import React, { useEffect, useRef, useState } from 'react'
import Editor from './components/Editor'

type WorkspaceFiles = Record<string, string>
const STORAGE_KEY = 'payjar-ide-workspace-v1'
const starterCode = 'public class main(@self) {\n  let total = 2 + 3;\n  println("Hello from Payjar!");\n  println(total);\n}\n'

function loadWorkspace(): WorkspaceFiles {
  try {
    const saved = window.localStorage.getItem(STORAGE_KEY)
    if (saved) {
      const parsed = JSON.parse(saved) as WorkspaceFiles
      if (parsed && typeof parsed === 'object' && Object.keys(parsed).length > 0) return parsed
    }
  } catch {
    // Start with the sample project when browser storage is unavailable.
  }
  return { 'main.pj': starterCode, 'examples/variables.pj': 'public class main(@self) {\n  let name = "Payjar";\n  println("Hello, " + name);\n}\n' }
}

function fileLabel(path: string) {
  return path.split('/').pop() || path
}

export default function App() {
  const [files, setFiles] = useState<WorkspaceFiles>(loadWorkspace)
  const [activePath, setActivePath] = useState('main.pj')
  const [filter, setFilter] = useState('')
  const [notice, setNotice] = useState('Workspace ready')
  const importRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(files))
      setNotice('All changes saved locally')
    } catch {
      setNotice('Browser storage unavailable — export files to keep a copy')
    }
  }, [files])

  useEffect(() => {
    if (!(activePath in files)) setActivePath(Object.keys(files)[0] || '')
  }, [activePath, files])

  const paths = Object.keys(files).sort((a, b) => a.localeCompare(b))
  const visiblePaths = paths.filter(path => path.toLowerCase().includes(filter.toLowerCase()))
  const activeCode = files[activePath] ?? ''

  function updateCode(value: string) {
    setFiles(current => ({ ...current, [activePath]: value }))
  }

  function createFile() {
    const entered = window.prompt('New file name (for example, src/main.pj):', 'untitled.pj')
    if (!entered) return
    const path = entered.trim().replace(/^\/+/, '')
    if (!path || path.split('/').some(part => !part || part === '.' || part === '..')) {
      setNotice('Use a valid relative file path')
      return
    }
    if (path in files) {
      setNotice('That file already exists')
      setActivePath(path)
      return
    }
    setFiles(current => ({ ...current, [path]: '' }))
    setActivePath(path)
    setNotice('Created ' + path)
  }

  function deleteFile(path: string) {
    if (paths.length <= 1) {
      setNotice('Keep at least one file in the workspace')
      return
    }
    if (!window.confirm('Delete ' + path + ' from this browser workspace?')) return
    setFiles(current => {
      const next = { ...current }
      delete next[path]
      return next
    })
    setNotice('Deleted ' + path)
  }

  function downloadFile(path = activePath) {
    const contents = files[path]
    if (contents === undefined) return
    const blob = new Blob([contents], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = fileLabel(path)
    anchor.click()
    URL.revokeObjectURL(url)
    setNotice('Exported ' + path)
  }

  function exportWorkspace() {
    const blob = new Blob([JSON.stringify({ name: 'Payjar workspace', files }, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = 'payjar-workspace.json'
    anchor.click()
    URL.revokeObjectURL(url)
    setNotice('Workspace exported')
  }

  async function importFiles(event: React.ChangeEvent<HTMLInputElement>) {
    const selected = Array.from(event.target.files || [])
    if (!selected.length) return
    const imported: WorkspaceFiles = {}
    for (const file of selected) {
      const name = file.name.replace(/[\\]/g, '/').split('/').pop() || 'imported.pj'
      imported[name] = await file.text()
    }
    setFiles(current => ({ ...current, ...imported }))
    setActivePath(Object.keys(imported)[0])
    setNotice('Imported ' + selected.length + (selected.length === 1 ? ' file' : ' files'))
    event.target.value = ''
  }

  function importWorkspace(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    if (!file) return
    file.text().then(text => {
      try {
        const parsed = JSON.parse(text)
        if (!parsed.files || typeof parsed.files !== 'object' || Array.isArray(parsed.files)) throw new Error('Invalid workspace')
        const safeFiles: WorkspaceFiles = {}
        for (const [path, content] of Object.entries(parsed.files)) {
          if (typeof content === 'string' && path && !path.split('/').some((part: string) => part === '..' || part === '')) safeFiles[path] = content
        }
        if (!Object.keys(safeFiles).length) throw new Error('No valid files found')
        setFiles(safeFiles)
        setActivePath(Object.keys(safeFiles)[0])
        setNotice('Workspace imported')
      } catch {
        setNotice('Could not import workspace JSON')
      }
      event.target.value = ''
    })
  }

  return (
    <div className="app-root">
      <aside className="activity-rail" aria-label="Workspace">
        <div className="brand-mark" title="Payjar Studio">P</div>
        <button className="rail-button active" title="Explorer" aria-label="Explorer">▤</button>
        <button className="rail-button" title="Import files" aria-label="Import files" onClick={() => importRef.current?.click()}>＋</button>
        <a className="rail-button rail-link" title="Open GitHub repository" aria-label="Open GitHub repository" href="https://github.com/Femn0X/Payjar" target="_blank" rel="noreferrer">⌘</a>
        <div className="rail-bottom"><span className="connection-dot" /></div>
      </aside>

      <aside className="sidebar">
        <div className="sidebar-heading">
          <span>EXPLORER</span>
          <div className="sidebar-actions">
            <button title="New file" aria-label="New file" onClick={createFile}>＋</button>
            <button title="Import files" aria-label="Import files" onClick={() => importRef.current?.click()}>⇧</button>
            <button title="Export workspace" aria-label="Export workspace" onClick={exportWorkspace}>⇩</button>
            <button title="Import workspace JSON" aria-label="Import workspace JSON" onClick={() => document.getElementById('workspace-import')?.click()}>↥</button>
          </div>
        </div>
        <div className="project-heading"><span className="chevron">⌄</span><span>PAYJAR-WORKSPACE</span></div>
        <div className="file-search"><span>⌕</span><input value={filter} onChange={event => setFilter(event.target.value)} placeholder="Filter files..." aria-label="Filter files" /></div>
        <nav className="file-list" aria-label="Project files">
          {visiblePaths.map(path => (
            <div key={path} className={'file-row ' + (path === activePath ? 'selected' : '')}>
              <button className="file-select" onClick={() => setActivePath(path)} title={path}>
                <span className="file-icon">{path.endsWith('.pj') ? 'P' : '·'}</span><span className="file-name">{path}</span>
              </button>
              <button className="file-delete" onClick={() => deleteFile(path)} title={'Delete ' + path} aria-label={'Delete ' + path}>×</button>
            </div>
          ))}
          {visiblePaths.length === 0 && <div className="empty-files">No matching files</div>}
        </nav>
        <div className="sidebar-footer">
          <div className="footer-title">LOCAL WORKSPACE</div>
          <div className="footer-copy"><span className="connection-dot" /> Autosave enabled</div>
          <p>Your files stay in this browser until you export them.</p>
        </div>
      </aside>

      <main className="main-area">
        <header className="toolbar">
          <div className="breadcrumbs"><span>Payjar</span><span className="crumb-separator">/</span><strong>{activePath || 'No file selected'}</strong></div>
          <div className="toolbar-actions">
            <span className="runtime-pill"><span className="connection-dot" /> In-browser runtime</span>
            <button className="secondary-button" onClick={() => downloadFile()} disabled={!activePath}>Export file</button>
            <button className="primary-button" onClick={() => window.dispatchEvent(new CustomEvent('payjar:run'))} disabled={!activePath}><span>▶</span> Run</button>
          </div>
        </header>

        <div className="editor-tabs">
          {paths.map(path => (
            <button key={path} className={'editor-tab ' + (path === activePath ? 'active' : '')} onClick={() => setActivePath(path)}>
              <span className="file-icon">P</span>{fileLabel(path)}{path === activePath && <span className="tab-dot" />}
            </button>
          ))}
          <div className="tab-spacer" />
          <button className="icon-button" title="New file" aria-label="New file" onClick={createFile}>＋</button>
        </div>

        <section className="editor-area">
          {activePath ? <Editor key={activePath} code={activeCode} fileName={activePath} workspaceFiles={files} onChange={updateCode} notice={setNotice} /> : (
            <div className="welcome-state"><div className="welcome-logo">P</div><h2>Payjar Studio</h2><p>Create or import a file to start coding.</p><button className="primary-button" onClick={createFile}>＋ New file</button></div>
          )}
        </section>

        <footer className="statusbar">
          <div className="status-left"><span>◈ Payjar</span><span>{activePath.endsWith('.pj') ? 'Payjar' : 'Plain Text'}</span><span>{notice}</span></div>
          <div className="status-right"><span>UTF-8</span><span>Spaces: 2</span><span>Browser IDE</span></div>
        </footer>
      </main>
      <input ref={importRef} className="visually-hidden" type="file" multiple accept=".pj,.txt,.payjar,.json,.ts,.js,.md,*/*" onChange={importFiles} />
      <input id="workspace-import" className="visually-hidden" type="file" accept=".json,application/json" onChange={importWorkspace} aria-label="Import workspace JSON" />
    </div>
  )
}
