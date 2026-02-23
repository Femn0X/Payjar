import React, { useRef, useState } from 'react'
import Editor from '@monaco-editor/react'
import { PJRT, PJS } from '../../payjar'
import {PayjarTokens,PayjarConfig} from '../syntax';

export default function CodeEditor(){
  const editorRef = useRef(null)
  const instanceRef=useRef(null)
  const [output, setOutput] = useState([])
  
 function handleBeforeMount(monacoInstance) {
  monacoInstance.languages.register({ id: "Payjar" });
  // Corrected the function name: setMonarchTokensProvider
  monacoInstance.languages.setMonarchTokensProvider("Payjar", PayjarTokens); 
  monacoInstance.languages.setLanguageConfiguration("Payjar", PayjarConfig);
}
  function handleEditorDidMount(editor) {
    editorRef.current = editor
  }
  
  function handleRun(){
    if (!editorRef.current) return
    const code = editorRef.current.getValue()
    try {
      // Use PJRT to run and parse the code. PJRT now returns collected outputs.
      const runtime = new PJRT(code, false)
      const result = runtime.run_code(true)
      if (result && result.outputs) setOutput(prev => [...prev, ...result.outputs])
    } catch (e) {
      setOutput(prev => [...prev, `Error: ${e?.message ?? e}`])
    }
  }
  
  return (
    <div className="code-editor" style={{height: '100%', display: 'flex', flexDirection: 'column'}}>
      <div style={{display: 'flex', justifyContent: 'flex-end', padding: '6px', gap: '8px'}}>
        <button onClick={handleRun} className="runButt">Run PayJar</button>
      </div>
      <div style={{flex: 1}}>
        <Editor
          height="100%"
          defaultLanguage="Payjar"
          defaultValue={'public class  main(@self){\n println("Hallo");\n}'}
          beforeMount={handleBeforeMount}
          onMount={handleEditorDidMount}
          options={{ automaticLayout: true }}
        />
      </div>

      <div className="output-pane" style={{height: '140px', overflow: 'auto', borderTop: '1px solid #ddd', padding: '8px', background: '#0f0f12', color: '#e6e6e6', fontFamily: 'monospace', fontSize: '13px'}}>
        <strong>Output</strong>
        {output.length === 0 ? <div style={{opacity: 0.7}}>No output yet. Click Run (PayJar).</div> : (
          <div style={{marginTop: 6}}>
            {output.map((line, idx) => <div key={idx}>{line}</div>)}
          </div>
        )}
      </div>
    </div>
  );
}