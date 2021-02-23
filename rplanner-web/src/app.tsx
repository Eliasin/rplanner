import React, { useEffect, useState } from 'react';
import { Nav, ListGroup, Button } from 'react-bootstrap';
import { addNote, getNotes, Note, NoteID, NoteFragment, TextNote, setNote } from './api';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faPlus } from '@fortawesome/free-solid-svg-icons'

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

function parseNote(note: Note, note_id: NoteID): JSX.Element {
    let contents = note.content.reduce((elem: JSX.Element, fragment: NoteFragment) => {
        return <>{elem}{createFragmentElement(fragment)}</>;
    }, <></>);

    return <div id={'note-' + note_id} className='note' data-note-id={note_id} contentEditable>{contents}</div>;
}

type FunctionBarProps = {
    section: Section;
}

function FunctionBar(props: FunctionBarProps) {
    switch (props.section) {
            case 'Notes': {
                return (
                    <ListGroup className='function-bar'>
                        <ListGroup.Item><Button variant="dark" onClick={() => {
                            addNote({ content: [{ Text: 'New note...' }], date: new Date().toUTCString() });
                        }}><FontAwesomeIcon icon={faPlus} /></Button></ListGroup.Item>
                    </ListGroup>
                );
            }
    }
    return (
        <ListGroup className='function-bar'>
            <ListGroup.Item>Function Bar</ListGroup.Item>
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
}

function Notes(props: NotesProps) {
    const [notes, setNotes] = useState<Array<[NoteID, Note]>>([]);
    const [needRefresh, setNeedRefresh] = useState(false);
    const noteChangeTimers = props.noteChangeTimers;

    useEffect(() => {
        getNotes().then((response: any) => {
            setNotes(response);
            setNeedRefresh(false);
        });
    }, [needRefresh]);

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
        const noteElements = notes.reduce((elem: JSX.Element, note_pair: [NoteID, Note]) => {
            const [note_id, note] = note_pair;
            return <>{elem}{parseNote(note, note_id)}</>;
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

function addFunctionBar(component: JSX.Element, section: Section): JSX.Element {
    return <>{component}<FunctionBar section={section} /></>;
}

export function App() {
    const [section, setSection] = useState<Section>('Notes');

    const noteTimers = new Map();
    const addSidebar = createAddSidebar(setSection);

    switch (section) {
            case 'Notes': {
                return addFunctionBar(addSidebar(<Notes noteChangeTimers={noteTimers} />), section);
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
