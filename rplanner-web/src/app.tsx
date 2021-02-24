import React, { useEffect, useState } from 'react';
import { Nav, ListGroup, Button } from 'react-bootstrap';
import { addNote, getNotes, Note, NoteID, NoteFragment, TextNote, setNote, deleteNote } from './api';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faPlus, faTimes } from '@fortawesome/free-solid-svg-icons'

import './app.css';
import 'bootstrap/dist/css/bootstrap.min.css';

type SidebarProps = {
    setSection: (section: Section) => void;
};

function Sidebar(props: SidebarProps) {
    const sections = ['Notes', 'Todo', 'Calendar', 'Settings'] as Array<Section>;

    const sidebar = sections.reduce((sidebar: JSX.Element, section: Section) => {
        return <>{sidebar}<Nav.Link className="sidebar-element" event-key={section} onClick={() => props.setSection(section)} >{section}</Nav.Link></>;
    }, <></>);

    return (
        <Nav className="flex-column sidebar" defaultActiveKey="Notes">
            {sidebar}
        </Nav>
    );
}

type Section = 'Notes' | 'Todo' | 'Calendar' | 'Settings';

function createFragmentElement(fragment: NoteFragment): JSX.Element {
    if ("Text" in fragment) {
        return <div children={fragment.Text} />;
    } else if ("Image" in fragment) {
        return <img src={fragment.Image} alt='Note' />;
    }

    return <p />;
}

function parseNote(note: Note, note_id: NoteID, requestNoteRefresh: () => void): JSX.Element {
    let contents = note.content.reduce((elem: JSX.Element, fragment: NoteFragment) => {
        return <>{elem}{createFragmentElement(fragment)}</>;
    }, <></>);

    const noteElement =
        <div id={'note-' + note_id} className='note' data-note-id={note_id} contentEditable>{contents}</div>;
    const deleteButton = <Button size='sm' className='noteDelete' variant='danger' onClick={() => {
        deleteNote(note_id);
        requestNoteRefresh();
    }}><FontAwesomeIcon icon={faTimes} size='sm' /></Button>;

    return <div className='noteBlock'>{deleteButton}{noteElement}</div>;
}

type NoteFunctionBarProps = {
    requestNoteRefresh: () => void;
}

function NoteFunctionBar(props: NoteFunctionBarProps) {
    return (
        <ListGroup className='function-bar'>
            <ListGroup.Item><Button variant="dark" onClick={() => {
                addNote({ content: [{ Text: 'New note...' }], date: new Date().toUTCString() });
                props.requestNoteRefresh();
            }}><FontAwesomeIcon icon={faPlus} /></Button></ListGroup.Item>
        </ListGroup>
    );
}

type NoteChangeTimer = {
    maxTimerValue: number;
    ticksSinceLastChange: number;
};

type NoteChangeTimers = Map<NoteID, NoteChangeTimer>;

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

type NotesProps = {
    noteChangeTimers: NoteChangeTimers;
    notes: Array<[NoteID, Note]>;
    requestNoteRefresh: () => void;
}

function compareNotes(a: [NoteID, Note], b: [NoteID, Note]): number {
    return a[0] - b[0];
}

function Notes(props: NotesProps) {
    const notes = props.notes;
    const noteChangeTimers = props.noteChangeTimers;

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
            return <>{elem}{parseNote(note, note_id, props.requestNoteRefresh)}</>;
        }, <></>);

        return <div className='notes'>{noteElements}</div>;
    } else {
        return <div className='notes'></div>;
    }
}

function Todo() {
    return <p children={'Todo'} />;
}

function Calendar() {
    return <p children={'Calendar'} />;
}

function Settings() {
    return <p children={'Settings'} />;
}

function createAddSidebar(setSection: (section: Section) => void): (component: JSX.Element) => JSX.Element {
    return (component: JSX.Element) => {
        return <><Sidebar setSection={setSection} /><div className='main'>{component}</div></>;
    };
}

function addNoteFunctionBar(component: JSX.Element, requestNoteRefresh: () => void): JSX.Element {
    return <>{component}<NoteFunctionBar requestNoteRefresh={requestNoteRefresh} /></>;
}

type AppProps = {
    noteTimers: NoteChangeTimers;
};

export function App(props: AppProps) {
    const [section, setSection] = useState<Section>('Notes');
    const [notes, setNotes] = useState<Array<[NoteID, Note]>>([]);
    const [needRefresh, setNeedRefresh] = useState(false);
    const noteTimers = props.noteTimers;

    const requestNoteRefresh = () => {
        setNeedRefresh(true);
    };

    useEffect(() => {
        getNotes().then((response: any) => {
            setNotes(response);
            setNeedRefresh(false);
        });
    }, [needRefresh]);

    const addSidebar = createAddSidebar(setSection);

    switch (section) {
            case 'Notes': {
                return addNoteFunctionBar(addSidebar(<Notes noteChangeTimers={noteTimers} notes={notes} requestNoteRefresh={requestNoteRefresh} />), requestNoteRefresh);
            }
            case 'Todo': {
                return addSidebar(<Todo />);
            }
            case 'Calendar': {
                return addSidebar(<Calendar />);
            }
            case 'Settings': {
                return addSidebar(<Settings />);
            }
    }
    return <></>;
}
