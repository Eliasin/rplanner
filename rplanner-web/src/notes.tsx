import React, { useEffect, useState } from 'react';
import { ListGroup, Button, Modal, Form, Image } from 'react-bootstrap';
import { addNote, Note, NoteID, NoteFragment, TextNote, setNote, deleteNote, uploadImage, getImageList, GetImageListResponse, insertImageIntoNote, FragmentNum } from './api';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faPlus, faTimes, faImage } from '@fortawesome/free-solid-svg-icons'

function createFragmentElement(fragment: NoteFragment, noteID: NoteID, order: number): JSX.Element {
    if ("Text" in fragment) {
        return <div children={fragment.Text} className='note-text' data-order={order} data-note-id={noteID} contentEditable></div>;
    } else if ("Image" in fragment) {
        return <img className='note-image' src={`images/${fragment.Image}`} data-order={order} alt='Note' />;
    }

    return <p />;
}

function parseNote(note: Note, note_id: NoteID, requestNoteRefresh: () => void, openImageModal: () => void): JSX.Element {
    let contents = <></>;

    for (let i = 0; i < note.content.length; i++) {
        const fragment = note.content[i];
        const fragmentElement = createFragmentElement(fragment, note_id, i);
        contents = <>{contents}{fragmentElement}</>;
    }

    const noteElement =
        <div id={'note-' + note_id} className='note' data-note-id={note_id} >{contents}</div>;
    const deleteButton = <Button size='sm' className='noteDelete' variant='danger' onClick={() => {
        deleteNote(note_id);
        requestNoteRefresh();
    }}><FontAwesomeIcon icon={faTimes} size='sm' /></Button>;
    const addImageButton = <Button className='noteImage' size='sm' onClick={() => openImageModal()}><FontAwesomeIcon icon={faImage} size='sm'/></Button>;

    return <div className='noteBlock'>{addImageButton}{deleteButton}{noteElement}</div>;
}

type NoteFunctionBarProps = {
    requestNoteRefresh: () => void;
    setNoteModalState: (state: NoteModalState) => void;
}

export function NoteFunctionBar(props: NoteFunctionBarProps) {
    return (
        <ListGroup className='function-bar'>
            <ListGroup.Item><Button variant="dark" onClick={() => {
                addNote({ content: [{ Text: 'New note...' }], date: new Date().toUTCString() });
                props.requestNoteRefresh();
            }}><FontAwesomeIcon icon={faPlus} /></Button></ListGroup.Item>
            <ListGroup.Item><Button variant="dark" onClick={() => {
                props.setNoteModalState({ kind: 'image-upload' });
            }}><FontAwesomeIcon icon={faPlus} /><FontAwesomeIcon icon={faImage} /></Button></ListGroup.Item>
        </ListGroup>
    );
}

export type NoteChangeTimer = {
    maxTimerValue: number;
    ticksSinceLastChange: number;
};

export type NoteChangeTimers = Map<NoteID, NoteChangeTimer>;

function resetNoteChangeTimer(noteID: NoteID, noteTimers: NoteChangeTimers) {
    const timer = noteTimers.get(noteID);
    if (timer !== undefined) {
        timer.ticksSinceLastChange = 0;
    }
}

function constructNoteContentFromElement(noteElement: HTMLElement): Array<NoteFragment> {
    const contents: Array<NoteFragment> = [];
    for (const child of noteElement.childNodes) {
        if (child.nodeName === 'DIV') {
            contents.push({
                Text: child.textContent ?? ''
            });
        } else if (child.nodeName === 'IMG') {
            contents.push({
                Image: (child as HTMLImageElement).src.slice(document.URL.length + '/images/'.length - 1)
            });
        }
    }

    return contents;
}

function flushNoteChanges(noteElement: HTMLElement): Promise<void> {
    if (noteElement.dataset['noteId'] === undefined || noteElement.textContent === null) {
        return Promise.resolve();
    }

    const noteID = parseInt(noteElement.dataset['noteId']);

    if (isNaN(noteID)) {
        return Promise.resolve();
    }

    const note: Note = {
        content: constructNoteContentFromElement(noteElement),
        date: new Date().toUTCString()
    };

    return setNote(noteID, note).then(() => {});
}

function getNoteElementID(noteElement: HTMLElement): NoteID | null {
    if (noteElement.dataset['noteId'] === undefined) {
        return null;
    }

    const noteID = parseInt(noteElement.dataset['noteId']);

    if (isNaN(noteID)) {
        return null;
    }

    return noteID;
}

function resetNoteElementChangeTimer(noteElement: HTMLElement, noteTimers: NoteChangeTimers) {
    const noteID = getNoteElementID(noteElement);
    if (noteID !== null) {
        resetNoteChangeTimer(noteID, noteTimers);
    }
}

function handleEnterKeyInNote(noteTimers: NoteChangeTimers, e: Event) {
    e.preventDefault();

    const selection = window.getSelection();
    let anchorNode = null;
    let anchorOffset = 0;
    if (selection) {
        anchorNode = selection.anchorNode;
        anchorOffset = selection.anchorOffset;
    }

    const noteElement = e.target as HTMLElement;
    const noteText = noteElement.textContent;

    if (noteText) {
        const startText = noteText.slice(0, anchorOffset);
        const endText = noteText.slice(anchorOffset)
        noteElement.textContent = startText + '\n' + endText;

    }

    if (selection && anchorNode && noteElement) {
        const range = document.createRange();
        range.setStart(noteElement.childNodes[0], anchorOffset + 1);
        selection.removeAllRanges();
        selection.addRange(range);
    }

    resetNoteElementChangeTimer(noteElement, noteTimers);
}

function getNoteElementFragmentNum(noteElement: HTMLElement): FragmentNum | null {
    if (noteElement.dataset['order'] === undefined || noteElement.textContent === null) {
        return null;
    }

    const fragmentNum = parseInt(noteElement.dataset['order']);

    if (isNaN(fragmentNum)) {
        return null;
    }

    return fragmentNum;
}

function moveCaretPositionIntoNote(position: CaretPosition) {
    const textFragments = document.getElementsByClassName('note-text');
    for (const _textFragment of textFragments) {
        const textFragment = _textFragment as HTMLElement;
        const noteID = getNoteElementID(textFragment);
        const fragmentNum = getNoteElementFragmentNum(textFragment);

        if (noteID !== null && fragmentNum !== null && noteID === position.noteID && fragmentNum === position.fragmentNum) {
            const selection = window.getSelection();

            if (selection) {
                const range = document.createRange();
                const textNode = textFragment.childNodes[0];

                /* When we try to move the caret into a note fragment with no text,
                 * we need toset the start of the range to be the note element as opposed
                 * to what we do normally which is using the text node.
                 */
                if (textNode === undefined) {
                    range.setStart(textFragment, 0);
                    range.collapse();
                    selection.removeAllRanges();
                    selection.addRange(range);

                    /* For some ridiculous reason, when we change the selection something
                    * changes it to the start of the fragment, so we listen for the next
                    * selection change and override it once
                    * */
                    document.onselectionchange = () => {
                        const range = document.createRange();
                        range.setStart(textFragment, 0);
                        range.collapse();
                        selection.removeAllRanges();
                        selection.addRange(range);

                        document.onselectionchange = () => {};
                    };

                    return;
                } else {
                    range.setStart(textNode, position.index);
                    range.collapse();
                    selection.removeAllRanges();
                    selection.addRange(range);

                    /* For some ridiculous reason, when we change the selection something
                    * changes it to the start of the fragment, so we listen for the next
                    * selection change and override it once
                    * */
                    document.onselectionchange = () => {
                        const range = document.createRange();
                        range.setStart(textNode, position.index);
                        range.collapse();
                        selection.removeAllRanges();
                        selection.addRange(range);

                        document.onselectionchange = () => {};
                    };

                    return;
                }
            }
        }
    }
}

function getNoteFromNotes(noteID: NoteID, notes: Array<[NoteID, Note]>): Note | null {
    for (const [id, note] of notes) {
        if (noteID === id) {
            return note;
        }
    }

    return null;
}

function getLastTextFragmentNum(noteID: NoteID, notes: Array<[NoteID, Note]>): FragmentNum | null {
    const note = getNoteFromNotes(noteID, notes);
    if (note === null) {
        return null;
    }

    for (let index = 0; index < note.content.length; index++) {
        const fragment = note.content[index];
        if ('Text' in fragment) {
            return index;
        }
    }

    return null;
}

function getNextTextFragmentNum(noteID: NoteID, notes: Array<[NoteID, Note]>): FragmentNum | null {
    const note = getNoteFromNotes(noteID, notes);
    if (note === null || note.content.length === 0) {
        return null;
    }

    for (let index = note.content.length - 1; index > 0; index--) {
        const fragment = note.content[index];
        if ('Text' in fragment) {
            return index;
        }
    }

    return null;
}


function getFragmentLength(note: Note, fragmentNum: FragmentNum): number | null {
    if (note.content.length <= fragmentNum) {
        return null;
    }

    const fragment = note.content[fragmentNum];

    if ('Text' in fragment) {
        return fragment.Text.length;
    }

    return null;
}

function handleUpArrowInNote(e: Event, notes: Array<[NoteID, Note]>) {
    const position = getCaretPosition();
    const noteID = getNoteElementID(e.target as HTMLElement);

    if (position && noteID) {
        const atBeginningOfFragment = position.index === 0;

        if (atBeginningOfFragment) {

            const lastTextFragmentNum = getLastTextFragmentNum(noteID, notes);

            if (lastTextFragmentNum !== null) {
                const note = getNoteFromNotes(noteID, notes);
                if (note === null) {
                    return;
                }

                const fragmentLength = getFragmentLength(note, lastTextFragmentNum);
                if (fragmentLength !== null && fragmentLength > 0) {
                    moveCaretPositionIntoNote({
                        noteID,
                        fragmentNum: lastTextFragmentNum,
                        index: fragmentLength,
                    });
                }
            }
        }

    }
}

function handleDownArrowInNote(e: Event, notes: Array<[NoteID, Note]>) {
    const position = getCaretPosition();
    const noteID = getNoteElementID(e.target as HTMLElement);

    if (position && noteID) {
        const note = getNoteFromNotes(noteID, notes);
        if (note === null) {
            return;
        }
        const fragmentLength = getFragmentLength(note, position.fragmentNum);
        if (fragmentLength === null) {
            return;
        }

        const atEndOfFragment = position.index === fragmentLength;
        if (atEndOfFragment) {
            const nextTextFragmentNum = getNextTextFragmentNum(noteID, notes);
            if (nextTextFragmentNum === null) {
                return;
            }

            moveCaretPositionIntoNote({
                noteID,
                fragmentNum: nextTextFragmentNum,
                index: 0,
            });
        }

    }
}

function createNoteKeyListener(noteTimers: NoteChangeTimers, notes: Array<[NoteID, Note]>): (e: Event) => void {
    return (e: Event) => {
        if (e.type !== 'keydown') {
            return;
        }

        if ((e as KeyboardEvent).key === "Enter") {
            handleEnterKeyInNote(noteTimers, e);
        } else if ((e as KeyboardEvent).key === 'ArrowUp') {
            handleUpArrowInNote(e, notes);
        } else if ((e as KeyboardEvent).key === 'ArrowDown') {
            handleDownArrowInNote(e, notes);
        }
    };
}

function createNoteEventListener(noteTimers: NoteChangeTimers): (e: Event) => void {
    return (e: Event) => {
        if (e.type !== 'input') {
            return;
        }

        const noteElement = e.target as HTMLElement;
        resetNoteElementChangeTimer(noteElement, noteTimers);
    };
}

function updateNoteChangeTimers(notes: Array<[NoteID, Note]>, noteTimers: NoteChangeTimers) {
    const needAdd = [];
    const ids = [];
    for (const [id, ] of notes) {
        if (!noteTimers.has(id)) {
            needAdd.push(id);
        }
        ids.push(id);
    }

    const needDelete = [];
    for (const [id, ] of noteTimers) {
        if (!ids.includes(id)) {
            needDelete.push(id);
        }
    }

    for (const id of needAdd) {
        noteTimers.set(id, {
            maxTimerValue: 2,
            ticksSinceLastChange: 2
        });
    }

    for (const id of needDelete) {
        noteTimers.delete(id);
    }
}

function tickNoteTimers(noteTimers: Map<number, NoteChangeTimer>) {
    for (const [id, timer] of noteTimers) {
        if (timer.ticksSinceLastChange === timer.maxTimerValue) {
            continue;
        }

        timer.ticksSinceLastChange += 1;

        if (timer.ticksSinceLastChange === timer.maxTimerValue) {
            const noteElement = document.getElementById('note-' + id);
            if (noteElement !== null) {
                flushNoteChanges(noteElement);
            }
        }
    }
}

function compareNotes(a: [NoteID, Note], b: [NoteID, Note]): number {
    return a[0] - b[0];
}

type ImageModalState = { kind: 'image-open'; };

type ImageUploadModalState = { kind: 'image-upload' };

export type NoteModalState = ImageModalState | ImageUploadModalState | { kind: 'closed' };

type NoteModalProps = {
    state: NoteModalState;
    requestNoteRefresh: () => void;
    hideNoteModal: () => void;
};

type NotesProps = {
    noteChangeTimers: NoteChangeTimers;
    notes: Array<[NoteID, Note]>;
    requestNoteRefresh: () => void;
    noteModalState: NoteModalState;
    setNoteModalState: (state: NoteModalState) => void;
}

function insertImageAtCaret(image: string) {
    const caretPosition = getCaretPosition();
    if (!caretPosition) {
        return;
    }

    const { noteID, fragmentNum, index } = caretPosition;

    const noteElement = document.getElementById(`note-${noteID}`);

    if (noteElement) {
        flushNoteChanges(noteElement).then(() => {
            insertImageIntoNote(noteID, fragmentNum, index, image);
        });
    }
}

type ImageViewerProps = {
    requestNoteRefresh: () => void;
};

function ImageViewer(props: ImageViewerProps) {
    const [imageNames, setImageNames] = useState<Array<string>>([]);

    useEffect(() => {
        getImageList().then((response: GetImageListResponse) => {
            setImageNames(response.images);
        });
    }, [setImageNames]);

    if (imageNames.length > 0) {
        const imageElements = imageNames.reduce((elem: JSX.Element, name: string) => {
            return <>{elem}
                <button className='image-selection' onClick={() => {
                    insertImageAtCaret(name);
                    props.requestNoteRefresh();
                }}><Image className='image-thumbnail' src={`images/${name}`} alt={`Thumbnail of ${name}`} thumbnail /></button>
                </>;
        }, <></>);

        return <div className='image-viewer'>{imageElements}</div>;
    }

    return <div className='image-viewer'/>;

}

function NoteModal(props: NoteModalProps) {
    switch (props.state.kind) {
            case 'image-open': {
                return <Modal dialogClassName='image-open-modal' contentClassName='image-open-modal' show={true} onHide={props.hideNoteModal}>
                        <Modal.Title>Select an Image</Modal.Title>
                        <Modal.Body>
                        <ImageViewer requestNoteRefresh={props.requestNoteRefresh} />
                        </Modal.Body>
                        </Modal>;
            }
            case 'image-upload': {
                return <Modal show={true} onHide={props.hideNoteModal}>
                    <Modal.Header>
                    <Modal.Title>Upload Image</Modal.Title>
                    </Modal.Header>
                    <Modal.Body>
                    <Form>
                    <Form.Label>Image File</Form.Label>
                    <Form.Control type='file' id='imageUploadInput' />
                    <Form.Label>Save Image as...</Form.Label>
                    <Form.Control type='text' id='imageUploadFilenameInput' placeholder='Filename...' />
                    </Form>
                    </Modal.Body>
                    <Modal.Footer>
                    <Button onClick={() => {
                        const fileInput = document.getElementById('imageUploadInput') as HTMLInputElement;
                        const fileNameInput = document.getElementById('imageUploadFilenameInput') as HTMLInputElement;

                        const files = fileInput.files;
                        if (files !== null && files.length > 0) {
                            const file = files[0];

                            if (fileNameInput.value === '') {
                                uploadImage(file.name, file);
                            } else {
                                uploadImage(fileNameInput.value, file);
                            }
                            props.hideNoteModal();
                        }

                    }}>Upload</Button>
                    </Modal.Footer>
                    </Modal>;
            }
    }

    return <Modal show={false} />;
}

type CaretPosition = {
    noteID: NoteID;
    fragmentNum: FragmentNum;
    index: number;
}

function getCaretPosition(): CaretPosition | null {
    const selection = window.getSelection();
    if (!selection) {
        return null;
    }

    const anchorNode = selection.anchorNode;
    if (!anchorNode) {
        return null;
    }

    const anchorOffset = selection.anchorOffset;

    let noteElement = anchorNode as HTMLElement;
    /* When we are in a fragment with text, the anchor node will be
     * the text node inside the note element. However, when the note
     * fragment has no text, the anchor node is just the note element.
     */
    if (anchorNode.nodeType === Node.TEXT_NODE) {
        noteElement = anchorNode.parentElement as HTMLElement;
    }
    const noteID = getNoteElementID(noteElement);

    if (noteID === null) {
        return null;
    }

    const fragmentNum = noteElement.dataset['order'];
    if (fragmentNum === undefined) {
        return null;
    }

    return {
        noteID,
        fragmentNum: parseInt(fragmentNum),
        index: anchorOffset
    };
}

export function Notes(props: NotesProps) {
    const notes = props.notes;
    const noteChangeTimers = props.noteChangeTimers;
    const { noteModalState, setNoteModalState } = props;
    const openImageModal = () => setNoteModalState({ kind: 'image-open' });

    useEffect(() => {
        updateNoteChangeTimers(notes, noteChangeTimers);

        const intervalID = setInterval(() => {
            tickNoteTimers(noteChangeTimers);
        }, 1000);

        return () => {
            clearInterval(intervalID);
        };
    }, [notes, noteChangeTimers]);

    useEffect(() => {
        let noteElements = Array.from(document.getElementsByClassName('note-text'));

        const noteEventListener = createNoteEventListener(noteChangeTimers);
        const noteEnterListener = createNoteKeyListener(noteChangeTimers, notes);
        for (const element of noteElements) {
            element.removeEventListener('input', noteEventListener);
            element.removeEventListener('keydown', noteEnterListener);
            element.addEventListener('input', noteEventListener);
            element.addEventListener('keydown', noteEnterListener);
        }
    }, [notes, noteChangeTimers]);


    if (notes.length !== 0) {
        const noteElements = notes.sort(compareNotes).reduce((elem: JSX.Element, note_pair: [NoteID, Note]) => {
            const [note_id, note] = note_pair;
            return <>{elem}{parseNote(note, note_id, props.requestNoteRefresh, openImageModal)}</>;
        }, <></>);

        return <><NoteModal state={noteModalState} hideNoteModal={() => setNoteModalState({kind: 'closed'})} requestNoteRefresh={props.requestNoteRefresh} /><div className='notes'>{noteElements}</div></>;
    } else {
        return <><div className='notes'></div></>;
    }
}
