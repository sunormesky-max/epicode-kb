import * as Y from 'yjs'

export type YDoc = Y.Doc

export function createCollabDoc(initialContent = ''): Y.Doc {
  const doc = new Y.Doc()
  const text = doc.getText('content')
  if (initialContent) {
    text.insert(0, initialContent)
  }
  return doc
}

export function getDocContent(doc: Y.Doc): string {
  return doc.getText('content').toString()
}

export function updateDocContent(doc: Y.Doc, content: string): void {
  const text = doc.getText('content')
  text.delete(0, text.length)
  text.insert(0, content)
}
