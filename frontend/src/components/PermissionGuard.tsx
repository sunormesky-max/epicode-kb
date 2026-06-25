import { ReactNode } from 'react'

interface PermissionGuardProps {
  children: ReactNode
  fallback?: ReactNode
}

export default function PermissionGuard({ children, fallback = null }: PermissionGuardProps) {
  const token = localStorage.getItem('access_token')
  if (!token) {
    return <>{fallback}</>
  }
  return <>{children}</>
}
