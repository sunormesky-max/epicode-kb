import { NavLink, Outlet } from 'react-router-dom'

const navItems = [
  { path: '/', label: 'Home' },
  { path: '/upload', label: 'Upload' },
  { path: '/search', label: 'Search' },
  { path: '/review', label: 'Review' },
  { path: '/conflicts', label: 'Conflicts' },
  { path: '/health', label: 'Health' },
  { path: '/graph', label: 'Graph' },
]

export default function Layout() {
  return (
    <div className="min-h-screen flex flex-col">
      {/* Sidebar */}
      <aside className="w-60 bg-slate-900 text-white flex-shrink-0 fixed inset-y-0 left-0 hidden md:block">
        <div className="p-6">
          <h1 className="text-xl font-bold flex items-center gap-2">
            <span className="text-2xl">🧠</span>
            epicode-kb
          </h1>
          <p className="text-xs text-slate-400 mt-1">v0.3.0</p>
        </div>
        <nav className="px-3">
          {navItems.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === '/'}
              className={({ isActive }) =>
                `block px-4 py-2.5 rounded-lg text-sm font-medium transition-colors mb-1 ${
                  isActive
                    ? 'bg-slate-700 text-white'
                    : 'text-slate-400 hover:text-white hover:bg-slate-800'
                }`
              }
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
      </aside>

      {/* Mobile top bar */}
      <div className="md:hidden bg-slate-900 text-white p-4 flex items-center gap-4">
        <h1 className="text-lg font-bold">🧠 epicode-kb</h1>
        <nav className="flex gap-2 ml-auto">
          {navItems.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === '/'}
              className={({ isActive }) =>
                `px-2 py-1 text-xs rounded ${isActive ? 'bg-slate-700' : 'text-slate-400'}`
              }
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
      </div>

      {/* Main content */}
      <main className="flex-1 md:ml-60 p-6 md:p-8">
        <Outlet />
      </main>
    </div>
  )
}
