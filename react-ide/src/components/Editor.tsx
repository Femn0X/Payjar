import React, { useEffect, useRef, useState } from 'react'
import MonacoEditor from '@monaco-editor/react'
import { PJRT } from '../../payjar'
import { PayjarTokens, PayjarConfig } from '../syntax'

type EditorProps = {
  code: string
  fileName: string
  workspaceFiles: Record<string, string>
  onChange: (value: string) => void
  notice: (message: string) => void
}

export default function CodeEditor({ code, fileName, workspaceFiles, onChange, notice }: EditorProps) {
  const editorRef = useRef<any>(null)
  const [output, setOutput] = useState<string[]>([])
  const [activePanel, setActivePanel] = useState<'terminal' | 'problems'>('terminal')
  const [running, setRunning] = useState(false)
  const [lineInfo, setLineInfo] = useState({ line: 1, column: 1 })

  function handleBeforeMount(monaco: any) {
    monaco.languages.register({ id: 'Payjar' })
    monaco.languages.setMonarchTokensProvider('Payjar', PayjarTokens)
    monaco.languages.setLanguageConfiguration('Payjar', PayjarConfig)
    monaco.editor.defineTheme('payjar-dark', {
      base: 'vs-dark',
      inherit: true,
      rules: [
        { token: 'keyword', foreground: 'C4B5FD' },
        { token: 'string', foreground: 'A7F3D0' },
        { token: 'number', foreground: 'FBBF24' },
        { token: 'comment', foreground: '718096', fontStyle: 'italic' },
      ],
      colors: {
        'editor.background': '#11131b',
        'editorLineNumber.foreground': '#51576a',
        'editorLineNumber.activeForeground': '#a5b4fc',
        'editorCursor.foreground': '#a5b4fc',
        'editor.selectionBackground': '#353957',
        'editor.lineHighlightBackground': '#191c27',
      },
    })
  }

  function handleEditorDidMount(editor: any) {
    editorRef.current = editor
    editor.onDidChangeCursorPosition((event: any) => {
      setLineInfo({ line: event.position.lineNumber, column: event.position.column })
    })
    editor.addCommand(2048 | 3, () => runCode())
  }

  function runCode() {
    if (running) return
    setRunning(true)
    setActivePanel('terminal')
    setOutput([])
    notice('Running ' + fileName + '…')
    try {
      const runtime = new PJRT(code, false, undefined, workspaceFiles, fileName)
      const result = runtime.run_code(false, (prompt?: string) => {
        const answer = window.prompt(prompt || 'Payjar program input:')
        return answer === null ? '' : answer
      })
      setOutput(result.outputs.length ? result.outputs : ['Program finished with no output.'])
      notice('Finished running ' + fileName)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setOutput(['Execution failed', message])
      notice('Execution failed')
    } finally {
      setRunning(false)
    }
  }

  useEffect(() => {
    const onRun = () => runCode()
    window.addEventListener('payjar:run', onRun)
    return () => window.removeEventListener('payjar:run', onRun)
  }, [code, fileName, running])

  function clearOutput() {
    setOutput([])
    notice('Output cleared')
  }

  return (
    <div className="ide-workbench">
      <div className="editor-toolbar">
        <div className="editor-toolbar-left">
          <span className="language-dot" />
          <span className="language-label">Payjar</span>
          <span className="toolbar-divider">/</span>
          <span className="toolbar-file">{fileName}</span>
        </div>
        <div className="editor-toolbar-right">
          <span className="shortcut-hint">Ctrl + Enter</span>
          <button className="primary-button run-button" onClick={runCode} disabled={running}>
            <span>{running ? '◌' : '▶'}</span>{running ? 'Running…' : 'Run Code'}
          </button>
        </div>
      </div>

      <div className="monaco-wrap">
        <MonacoEditor
          height="100%"
          language="Payjar"
          theme="payjar-dark"
          value={code}
          beforeMount={handleBeforeMount}
          onMount={handleEditorDidMount}
          onChange={value => onChange(value ?? '')}
          options={{
            automaticLayout: true,
            minimap: { enabled: false },
            fontSize: 14,
            fontFamily: "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
            lineHeight: 24,
            padding: { top: 18, bottom: 16 },
            scrollBeyondLastLine: false,
            roundedSelection: true,
            renderLineHighlight: 'all',
            cursorBlinking: 'smooth',
            smoothScrolling: true,
            tabSize: 2,
            wordWrap: 'on',
            suggestOnTriggerCharacters: true,
            quickSuggestions: true,
            glyphMargin: false,
            folding: true,
          }}
        />
      </div>

      <section className="bottom-panel">
        <div className="panel-heading">
          <div className="panel-tabs">
            <button className={activePanel === 'terminal' ? 'panel-tab active' : 'panel-tab'} onClick={() => setActivePanel('terminal')}>TERMINAL <span className="panel-count">{output.length}</span></button>
            <button className={activePanel === 'problems' ? 'panel-tab active' : 'panel-tab'} onClick={() => setActivePanel('problems')}>PROBLEMS</button>
          </div>
          <div className="panel-actions">
            <span className="terminal-context">Payjar Runtime</span>
            <button className="icon-button" title="Clear output" aria-label="Clear output" onClick={clearOutput}>⌫</button>
          </div>
        </div>
        {activePanel === 'terminal' ? (
          <div className="terminal-output" role="log" aria-live="polite">
            {output.length === 0 ? (
              <div className="terminal-empty"><span className="terminal-prompt">›</span><span>Output from your program will appear here. Press <kbd>Run Code</kbd> to execute.</span></div>
            ) : output.map((line, index) => (
              <div key={index} className={'output-line ' + (line === 'Execution failed' ? 'output-error' : '')}>
                <span className="output-gutter">{String(index + 1).padStart(2, '0')}</span><span>{line}</span>
              </div>
            ))}
          </div>
        ) : (
          <div className="problems-empty"><span className="success-mark">✓</span>{output.some(line => line === 'Execution failed') ? 'Execution reported an error. Check the Terminal tab.' : 'No problems reported by the last run.'}</div>
        )}
      </section>
      <div className="editor-status"><span>Ln {lineInfo.line}, Col {lineInfo.column}</span><span>Spaces: 2</span><span>UTF-8</span><span className="status-success"><span className="connection-dot" /> Ready</span></div>
    </div>
  )
}
