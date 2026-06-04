// Sidebar — fixed 250px navigation rail for the app shell.
//
// Renders the two top-level tabs ("Resource Overview" / "Terminal") and reports
// the selected one to the parent via onSelect. It holds no state itself: the
// active tab is owned by the App shell so the layout stays a single source of
// truth (mirrors the stateless RangeSelector pattern in F5).

export type Tab = 'overview' | 'terminal'

interface SidebarItem {
  id: Tab
  label: string
}

const ITEMS: SidebarItem[] = [
  { id: 'overview', label: 'Resource Overview' },
  { id: 'terminal', label: 'Terminal' },
]

export interface SidebarProps {
  active: Tab
  onSelect: (tab: Tab) => void
}

export default function Sidebar({ active, onSelect }: SidebarProps) {
  return (
    <aside className="flex h-full flex-col border-r border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800">
      <div className="px-4 py-5">
        <span className="text-lg font-semibold text-gray-900 dark:text-gray-100">Dobby</span>
      </div>
      <nav aria-label="Primary" className="flex flex-col gap-1 px-2">
        {ITEMS.map((item) => {
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
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-gray-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700')
              }
            >
              {item.label}
            </button>
          )
        })}
      </nav>
    </aside>
  )
}
