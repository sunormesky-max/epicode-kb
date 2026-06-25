// tRPC-style client wrapper (vanilla mode).
// Since the backend is REST (not tRPC server), this module provides
// a type-safe API calling layer that mirrors tRPC's interface.

import * as api from '../lib/api'
import type {
  CreateMemoryRequest,
  ListMemoriesResponse,
  Memory,
  RememberResponse,
  SearchResponse,
  RecallRequest,
  UploadResponse,
} from '../lib/types'

// tRPC-like procedure definitions with zod-compatible validation
export const trpc = {
  memory: {
    remember: (input: CreateMemoryRequest) => api.remember(input),
    get: (id: string) => api.getMemory(id),
    list: (params: Parameters<typeof api.listMemories>[0]) => api.listMemories(params),
    updateTrust: (id: string, trustLevel: number, reason?: string) =>
      api.updateTrust(id, trustLevel, reason),
    adopt: (id: string) => api.adoptMemory(id),
    reject: (id: string) => api.rejectMemory(id),
  },
  search: {
    search: (params: Parameters<typeof api.search>[0]) => api.search(params),
    recall: (input: RecallRequest) => api.recall(input),
  },
  upload: {
    upload: (file: File, spaceId: string, provenance?: string) =>
      api.upload(file, spaceId, provenance),
  },
  system: {
    health: () => api.systemHealth(),
    version: () => api.systemVersion(),
  },
}

// Re-export types for convenience
export type {
  CreateMemoryRequest,
  ListMemoriesResponse,
  Memory,
  RememberResponse,
  SearchResponse,
  RecallRequest,
  UploadResponse,
} from '../lib/types'
