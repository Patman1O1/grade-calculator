import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { loadEngine } from './engine'

const root = createRoot(document.getElementById('root')!)

loadEngine().then(
  () =>
    root.render(
      <StrictMode>
        <App />
      </StrictMode>,
    ),
  (e: unknown) =>
    root.render(
      <main className="empty-state">
        <p className="banner" role="alert">
          The grade calculator couldn’t start: {e instanceof Error ? e.message : String(e)}. It needs a browser
          with WebAssembly support.
        </p>
      </main>,
    ),
)
