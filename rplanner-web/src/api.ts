export type TextNote = {
    Text: string;
}

export type ImageNote = {
    Image: string;
}

export type NoteFragment = TextNote | ImageNote;

export type Note = {
    content: Array<NoteFragment>;
    date: string;
}

export type NoteID = number;
export async function getNotes(): Promise<Array<[NoteID, Note]>> {
    return (await fetch('/api/notes')).json();
}

export type AddNoteResponse = {
    note_id: number;
};

export async function addNote(note: Note) {
    return (await fetch('/api/add_note', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(note)
    })).json();
}

export type SetNoteRequest = {
    note_id: number;
    note: Note;
};

export async function setNote(note_id: number, note: Note) {
    const setNoteRequest: SetNoteRequest = {
        note_id,
        note
    };

    return (await fetch('/api/set_note', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(setNoteRequest)
    }));
}

export type DeleteNoteRequest = {
    note_id: number;
};

export async function deleteNote(note_id: number) {
    const deleteNoteRequest: DeleteNoteRequest = {
        note_id,
    };

    return (await fetch('/api/delete_note', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(deleteNoteRequest),
    }));
}
