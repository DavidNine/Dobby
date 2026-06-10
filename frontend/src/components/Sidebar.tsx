// Sidebar — fixed 250px navigation rail for the app shell.
//
// Top section holds the primary tabs; "Settings" is pinned to the bottom
// (separated by a spacer) so it reads as a secondary, app-level destination.
// Stateless: the active tab is owned by the App shell and reported via
// onSelect (mirrors the RangeSelector pattern).

export type Tab = 'overview' | 'terminal' | 'docker' | 'settings'

interface SidebarItem {
  id: Tab
  label: string
}

const TOP_ITEMS: SidebarItem[] = [
  { id: 'overview', label: 'Resource Overview' },
  { id: 'terminal', label: 'Terminal' },
  { id: 'docker', label: 'Docker Containers' },
]

const BOTTOM_ITEMS: SidebarItem[] = [{ id: 'settings', label: 'Settings' }]

export interface SidebarProps {
  active: Tab
  onSelect: (tab: Tab) => void
}

export default function Sidebar({ active, onSelect }: SidebarProps) {
  const renderItem = (item: SidebarItem) => {
    const isActive = item.id === active
    return (
      <button
        key={item.id}
        type="button"
        aria-current={isActive ? 'page' : undefined}
        data-active={isActive}
        onClick={() => onSelect(item.id)}
        className={
          'rounded-md px-3 py-2 text-left text-sm font-medium transition-colors ' +
          (isActive
            ? 'bg-accent text-accent-contrast shadow'
            : 'text-muted hover:bg-background')
        }
      >
        {item.label}
      </button>
    )
  }

  return (
    <aside className="glass-surface flex h-full flex-col border-r border-border bg-card">
      <div className="px-4 py-5">
        <span className="text-lg font-semibold text-main">Dobby</span>
      </div>

      <nav aria-label="Primary" className="flex flex-col gap-1 px-2">
        {TOP_ITEMS.map(renderItem)}
      </nav>

      {/* Spacer pushes the settings group to the bottom of the rail. */}
      <nav
        aria-label="Settings"
        className="mt-auto flex flex-col gap-1 border-t border-border px-2 py-2"
      >
        {BOTTOM_ITEMS.map(renderItem)}
      </nav>
    </aside>
  )
}
