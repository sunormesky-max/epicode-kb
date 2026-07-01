import { useEffect, useState } from 'react'
import { authHeaders } from '../lib/auth'

interface User {
  id: string
  email: string
  name: string
  global_role: string
  is_active: boolean
}

export default function AdminConsole() {
  const [users, setUsers] = useState<User[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetch('/api/v1/auth/me', { headers: authHeaders() })
      .then(() => setLoading(false))
      .catch(() => setLoading(false))
  }, [])

  if (loading) {
    return <div className="p-8 text-gray-500">Loading...</div>
  }

  return (
    <div className="p-8 max-w-5xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">Admin Console</h1>
      <div className="bg-white rounded-lg shadow overflow-hidden">
        <table className="min-w-full divide-y divide-gray-200">
          <thead className="bg-gray-50">
            <tr>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Email</th>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Role</th>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-200">
            {users.map((u) => (
              <tr key={u.id}>
                <td className="px-6 py-4">{u.email}</td>
                <td className="px-6 py-4 capitalize">{u.global_role}</td>
                <td className="px-6 py-4">{u.is_active ? 'Active' : 'Inactive'}</td>
              </tr>
            ))}
            {users.length === 0 && (
              <tr>
                <td colSpan={3} className="px-6 py-8 text-center text-gray-500">
                  User management UI placeholder.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
