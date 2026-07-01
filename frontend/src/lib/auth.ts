const TOKEN_KEY = 'access_token'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token)
}

export function removeToken(): void {
  localStorage.removeItem(TOKEN_KEY)
}

export function isAuthenticated(): boolean {
  return !!getToken()
}

export function logout(): void {
  removeToken()
  window.location.href = '/login'
}

/// Single source of truth for the Authorization header. Pages/components
/// should import this instead of redefining it.
export function authHeaders(): Record<string, string> {
  const token = getToken()
  return token ? { Authorization: `Bearer ${token}` } : {}
}

/// User name/id for awareness coloring (collab cursors). Defaults to
/// anonymous when no profile is stored.
export function currentUserName(): string {
  return localStorage.getItem('user_name') || 'Anonymous'
}
export function currentUserId(): string {
  return localStorage.getItem('user_id') || 'anon'
}
