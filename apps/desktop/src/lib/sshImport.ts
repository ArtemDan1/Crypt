import type { SshCandidate } from './types'
export function defaultTitle(fileName: string, comment?: string): string {
  return comment?.trim() || fileName.replace(/^id_/, '')
}
export function selectable(candidate: SshCandidate): boolean {
  return !candidate.alreadyImported
}
