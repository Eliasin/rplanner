import React, { useEffect, useState } from 'react';
import { ListGroup, Button, Modal, Form, Image } from 'react-bootstrap';
import { addNote, Note, NoteID, NoteFragment, TextNote, setNote, deleteNote, uploadImage, getImageList, GetImageListResponse, insertImageIntoNote } from './api';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faPlus, faTimes, faImage } from '@fortawesome/free-solid-svg-icons'

function createFragmentElement(fragment: NoteFragment, noteID: NoteID, order: number): JSX.Element {
    if ("Text" in fragment) {
        return <div className='note-text' children={fragment.Text} data-order={order} data-note-id={noteID} contentEditable/>;
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

function flushNoteChanges(noteElement: HTMLElement): Promise<void> {
    if (noteElement.dataset['noteId'] === undefined || noteElement.textContent === undefined) {
        return Promise.resolve();
    }

    const noteID = parseInt(noteElement.dataset['noteId']);

    if (isNaN(noteID)) {
        return Promise.resolve();
    }

    const textFragment: TextNote = {
        Text: noteElement.textContent ?? ''
    };

    const note: Note = {
        content: [textFragment],
        date: new Date().toUTCString()
    };

    return setNote(noteID, note).then(() => {});
}

function getNoteElementID(noteElement: HTMLElement): NoteID | null {
    if (noteElement.dataset['noteId'] === undefined || noteElement.textContent === undefined) {
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

function createNoteEnterListener(noteTimers: NoteChangeTimers): (e: Event) => void {
    return (e: Event) => {
        if (e.type !== 'keydown') {
            return;
        }

        if ((e as KeyboardEvent).key === "Enter") {
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
    fragmentNum: number;
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

    const noteElement = anchorNode.parentElement as HTMLElement;
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
        const noteEnterListener = createNoteEnterListener(noteChangeTimers);
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
