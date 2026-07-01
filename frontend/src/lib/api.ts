// HTTP API wrapper — handles unified {code, data, message} response format

import { authHeaders } from './auth'
import type {
  ApiResponse,
  Conflict,
  ConflictResolution,
  CreateMemoryRequest,
  EditorContext,
  GraphData,
  ListMemoriesResponse,
  Memory,
  RememberResponse,
  SearchResponse,
  RecallRequest,
  UploadResponse,
} from './types'

const API_BASE = '/api/v1'

export class ApiError extends Error {
  code: number
  constructor(code: number, message: string) {
    super(message)
    this.code = code
    this.name = 'ApiError'
  }
}

async function request<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const url = `${API_BASE}${path}`

  // Always inject the Authorization header (single source of truth in lib/auth).
  const headers: Record<string, string> = { ...authHeaders(), ...(options.headers as Record<string, string> | undefined) }

  let response: Response
  try {
    response = await fetch(url, { ...options, headers })
  } catch (err) {
    throw new ApiError(-1, `Network error: ${err instanceof Error ? err.message : 'unknown'}`)
  }

  let body: ApiResponse<T>
  try {
    body = await response.json()
  } catch {
    throw new ApiError(-1, `Failed to parse response (status ${response.status})`)
  }

  if (body.code !== 0) {
    throw new ApiError(body.code, body.message)
  }

  return body.data
}

// ============================================================
// Memory API
// ============================================================

export async function remember(req: CreateMemoryRequest): Promise<RememberResponse> {
  return request<RememberResponse>('/remember', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  })
}

export async function getMemory(id: string): Promise<Memory> {
  return request<Memory>(`/memories/${id}`)
}

export async function listMemories(params: {
  space_id: string
  provenance?: string
  min_trust?: number
  review_status?: string
  limit?: number
  offset?: number
}): Promise<ListMemoriesResponse> {
  const query = new URLSearchParams({ space_id: params.space_id })
  if (params.provenance) query.set('provenance', params.provenance)
  if (params.min_trust !== undefined) query.set('min_trust', String(params.min_trust))
  if (params.review_status) query.set('review_status', params.review_status)
  if (params.limit !== undefined) query.set('limit', String(params.limit))
  if (params.offset !== undefined) query.set('offset', String(params.offset))
  return request<ListMemoriesResponse>(`/memories?${query}`)
}

export async function updateTrust(id: string, trustLevel: number, reason?: string): Promise<Memory> {
  return request<Memory>(`/memories/${id}/trust`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ trust_level: trustLevel, reason }),
  })
}

export async function adoptMemory(id: string): Promise<Memory> {
  return request<Memory>(`/memories/${id}/adopt`, { method: 'POST' })
}

export async function rejectMemory(id: string): Promise<Memory> {
  return request<Memory>(`/memories/${id}/reject`, { method: 'POST' })
}

// ============================================================
// Search API
// ============================================================

export async function search(params: {
  q: string
  space_id: string
  mode?: string
  min_trust?: number
  provenance?: string
  review_status?: string
  limit?: number
  offset?: number
}): Promise<SearchResponse> {
  const query = new URLSearchParams({
    q: params.q,
    space_id: params.space_id,
  })
  if (params.mode) query.set('mode', params.mode)
  if (params.min_trust !== undefined) query.set('min_trust', String(params.min_trust))
  if (params.provenance) query.set('provenance', params.provenance)
  if (params.review_status) query.set('review_status', params.review_status)
  if (params.limit !== undefined) query.set('limit', String(params.limit))
  if (params.offset !== undefined) query.set('offset', String(params.offset))
  return request<SearchResponse>(`/search?${query}`)
}

export async function recall(req: RecallRequest): Promise<SearchResponse> {
  return request<SearchResponse>('/recall', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  })
}

// ============================================================
// Upload API
// ============================================================

export async function upload(
  file: File,
  spaceId: string,
  provenance?: string,
): Promise<UploadResponse> {
  const formData = new FormData()
  formData.append('file', file)
  formData.append('space_id', spaceId)
  if (provenance) formData.append('provenance', provenance)

  const response = await fetch(`${API_BASE}/upload`, {
    method: 'POST',
    headers: authHeaders(),
    body: formData,
  })

  let body: ApiResponse<UploadResponse>
  try {
    body = await response.json()
  } catch {
    throw new ApiError(-1, `Failed to parse upload response (status ${response.status})`)
  }

  if (body.code !== 0) {
    throw new ApiError(body.code, body.message)
  }

  return body.data
}

// ============================================================
// System API
// ============================================================

export async function systemHealth(): Promise<{ status: string; version: string }> {
  return request('/system/health')
}

export async function systemVersion(): Promise<{ version: string; name: string }> {
  return request('/system/version')
}

// ============================================================
// Conflict center API (P3)
// ============================================================

export async function listConflicts(spaceId = 'sp_default'): Promise<Conflict[]> {
  return request<Conflict[]>(`/conflicts?space_id=${encodeURIComponent(spaceId)}`)
}

export async function resolveConflict(
  id: string,
  resolution: ConflictResolution,
): Promise<{ id: string; resolution: string; status: string }> {
  return request(`/conflicts/${id}/resolve`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ resolution }),
  })
}

// ============================================================
// Knowledge graph API (P3-4)
// ============================================================

export async function getGraph(spaceId = 'sp_default'): Promise<GraphData> {
  return request<GraphData>(`/graph?space_id=${encodeURIComponent(spaceId)}`)
}

// ============================================================
// Editor context API (P5-2)
// ============================================================

export async function getEditorContext(
  memoryId: string,
  cursor: string,
  spaceId = 'sp_default',
): Promise<EditorContext> {
  const query = new URLSearchParams({
    memory_id: memoryId,
    cursor,
    space_id: spaceId,
  })
  return request<EditorContext>(`/collab/context?${query}`)
}
