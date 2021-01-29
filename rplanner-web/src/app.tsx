import React, { useEffect, useState } from 'react';
import { Card, Nav, ListGroup, Button } from 'react-bootstrap';
import { addNote, getNotes, Note, NoteFragment } from './api';
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

function parseNote(note: Note): JSX.Element {
    let contents = note.content.reduce((elem: JSX.Element, fragment: NoteFragment) => {
        return <>{elem}{createFragmentElement(fragment)}</>;
    }, <></>);

    return <div className='note' contentEditable>{contents}</div>;
}

function AddNoteForm() {

}

type FunctionBarProps = {
    section: Section;
}

function FunctionBar(props: FunctionBarProps) {
    switch (props.section) {
            case 'Notes': {
                return (
                    <ListGroup className='function-bar'>
                        <ListGroup.Item><Button variant="dark"><FontAwesomeIcon icon={faPlus} /></Button></ListGroup.Item>
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

function noteInputEventListener(e: Event) {
    console.log(e)
}

function Notes() {
    const [notes, setNotes] = useState([]);

    useEffect(() => {
        getNotes().then((response: any) => {
            setNotes(response);
        });
    }, []);

    useEffect(() => {
        let notes = Array.from(document.getElementsByClassName('note'));

        for (const note of notes) {
            note.removeEventListener('input', noteInputEventListener);
            note.addEventListener('input', noteInputEventListener);
        }
    }, [notes]);

    if (notes.length !== 0) {
        const noteElements = notes.reduce((elem: JSX.Element, note: Note) => {
            return <>{elem}{parseNote(note)}</>;
        }, <></>);

        return <div className='notes'>{noteElements}</div>;
    } else {
        return <div className='notes'><button children={'ADD NOTE'} onClick={() => {
            addNote({ content: [{ Text: 'test' }], date: new Date().toUTCString() });
        }} /></div>;
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

    const addSidebar = createAddSidebar(setSection);

    switch (section) {
            case 'Notes': {
                return addFunctionBar(addSidebar(<Notes />), section);
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
