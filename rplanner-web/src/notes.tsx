import React, { useEffect } from 'react';
import { ListGroup, Button, Modal, Form } from 'react-bootstrap';
import { addNote, Note, NoteID, NoteFragment, TextNote, setNote, deleteNote, uploadImage } from './api';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faPlus, faTimes, faImage } from '@fortawesome/free-solid-svg-icons'

function createFragmentElement(fragment: NoteFragment): JSX.Element {
    if ("Text" in fragment) {
        return <div children={fragment.Text} />;
    } else if ("Image" in fragment) {
        return <img src={fragment.Image} alt='Note' />;
    }

    return <p />;
}

function parseNote(note: Note, note_id: NoteID, requestNoteRefresh: () => void, openImageModal: (noteID: NoteID) => void): JSX.Element {
    let contents = note.content.reduce((elem: JSX.Element, fragment: NoteFragment) => {
        return <>{elem}{createFragmentElement(fragment)}</>;
    }, <></>);

    const noteElement =
        <div id={'note-' + note_id} className='note' data-note-id={note_id} contentEditable>{contents}</div>;
    const deleteButton = <Button size='sm' className='noteDelete' variant='danger' onClick={() => {
        deleteNote(note_id);
        requestNoteRefresh();
    }}><FontAwesomeIcon icon={faTimes} size='sm' /></Button>;
    const addImageButton = <Button className='noteImage' size='sm' onClick={() => openImageModal(note_id)}><FontAwesomeIcon icon={faImage} size='sm'/></Button>;

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

function flushNoteChanges(noteElement: HTMLElement) {
    if (noteElement.dataset['noteId'] === undefined || noteElement.textContent === undefined) {
        return;
    }

    const noteID = parseInt(noteElement.dataset['noteId']);

    if (isNaN(noteID)) {
        return;
    }

    const textFragment: TextNote = {
        Text: noteElement.textContent ?? ''
    };

    const note: Note = {
        content: [textFragment],
        date: new Date().toUTCString()
    };

    setNote(noteID, note);
}

function createNoteInputEventListener(noteTimers: NoteChangeTimers): (e: Event) => void {
    return (e: Event) => {
        const noteElement = e.target as HTMLElement;

        if (noteElement.dataset['noteId'] === undefined || noteElement.textContent === undefined) {
            return;
        }

        const noteID = parseInt(noteElement.dataset['noteId']);

        if (isNaN(noteID)) {
            return;
        }

        resetNoteChangeTimer(noteID, noteTimers);
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

type ImageModalState = { kind: 'image-open'; id: NoteID };

type ImageUploadModalState = { kind: 'image-upload' };

export type NoteModalState = ImageModalState | ImageUploadModalState | { kind: 'closed' };

type NoteModalProps = {
    state: NoteModalState;
    hideNoteModal: () => void;
};

type NotesProps = {
    noteChangeTimers: NoteChangeTimers;
    notes: Array<[NoteID, Note]>;
    requestNoteRefresh: () => void;
    noteModalState: NoteModalState;
    setNoteModalState: (state: NoteModalState) => void;
}

function NoteModal(props: NoteModalProps) {
    switch (props.state.kind) {
            case 'image-open': {
                     return <Modal show={true} onHide={props.hideNoteModal}>
                        <Modal.Title>{props.state.id}</Modal.Title>
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
                        }
                    }}>Upload</Button>
                    </Modal.Footer>
                    </Modal>;
            }
    }

    return <Modal show={false} />;
}

export function Notes(props: NotesProps) {
    const notes = props.notes;
    const noteChangeTimers = props.noteChangeTimers;
    const { noteModalState, setNoteModalState } = props;
    const openImageModal = (id: NoteID) => setNoteModalState({ kind: 'image-open', id, });

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
        let noteElements = Array.from(document.getElementsByClassName('note'));

        const noteInputEventListener = createNoteInputEventListener(noteChangeTimers);
        for (const element of noteElements) {
            element.removeEventListener('input', noteInputEventListener);
            element.addEventListener('input', noteInputEventListener);
        }
    }, [notes, noteChangeTimers]);


    if (notes.length !== 0) {
        const noteElements = notes.sort(compareNotes).reduce((elem: JSX.Element, note_pair: [NoteID, Note]) => {
            const [note_id, note] = note_pair;
            return <>{elem}{parseNote(note, note_id, props.requestNoteRefresh, openImageModal)}</>;
        }, <></>);

        return <><NoteModal state={noteModalState} hideNoteModal={() => setNoteModalState({kind: 'closed'})} /><div className='notes'>{noteElements}</div></>;
    } else {
        return <><div className='notes'></div></>;
    }
}
