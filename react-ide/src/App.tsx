import React from 'react'
import Editor from './components/Editor'

export default function App() {
  return (
    <div className="app-root">
      <aside className="sidebar">
        <ul className="file-list">
          <li></li>
          <li></li>
          <li></li>
          <li></li>
        </ul>
      </aside>

      <main className="main-area">
        <header className="toolbar">
          <div className="title">Payjar IDE — React + Vite + Tauri</div>
          <div className="actions">Ready</div>
        </header>

        <section className="editor-area">
          <Editor />
        </section>

        <footer className="statusbar">Line 1, Col 1</footer>
      </main>
    </div>
  )
}
