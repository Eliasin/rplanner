import React, { useEffect, useState } from 'react';
import { Nav } from 'react-bootstrap';
import { getNotes, Note, NoteID } from './api';
import { NoteFunctionBar, NoteChangeTimers, Notes } from './notes';

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
