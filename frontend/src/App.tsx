// App shell — CSS Grid layout wrapping the existing Monitor screen.
//
// A two-column grid: a fixed 250px Sidebar on the left and the active view on
// the right. The Sidebar tab state lives here (the single source of truth) and
// switches the main panel between:
//   - "overview" → the unchanged F6 Resource Overview (Dashboard),
//   - "terminal" → the Terminal view,
//   - "docker"   → the Docker Containers table,
//   - "settings" → the Settings view (theme switcher).
//
// useTheme is called here (the root) so the saved theme applies on every tab.
import { useState } from 'react'
import Sidebar, { type Tab } from './components/Sidebar'
import Dashboard from './pages/Dashboard'
import Terminal from './pages/Terminal'
import DockerContainers from './pages/DockerContainers'
import Settings from './pages/Settings'
import { useTheme } from './hooks/useTheme'

function App() {
  const [tab, setTab] = useState<Tab>('overview')
  const { theme, setTheme } = useTheme()

  return (
    <div className="grid h-screen grid-cols-[250px_1fr]">
      <Sidebar active={tab} onSelect={setTab} />
      {/* min-w-0 lets the main column shrink instead of overflowing the grid;
          overflow-y-auto keeps the sidebar fixed while content scrolls. */}
      <main className="min-w-0 overflow-y-auto">
        {tab === 'overview' && <Dashboard />}
        {tab === 'terminal' && <Terminal />}
        {tab === 'docker' && <DockerContainers />}
        {tab === 'settings' && <Settings theme={theme} onChange={setTheme} />}
      </main>
    </div>
  )
}

export default App
