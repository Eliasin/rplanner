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

export async function getNotes(): Promise<Array<Note>> {
    return (await fetch('/api/notes')).json();
}

export async function addNote(note: Note) {
    fetch('/api/add_note', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(note)
    })
}
