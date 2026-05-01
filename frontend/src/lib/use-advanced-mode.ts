import { useSyncExternalStore } from 'react'

const STORAGE_KEY = 'pennywise.advancedMode'
const CHANGE_EVENT = 'pennywise:advanced-mode-changed'

function getSnapshot(): boolean {
  return typeof window !== 'undefined' && window.localStorage.getItem(STORAGE_KEY) === '1'
}

function subscribe(onChange: () => void): () => void {
  const onStorage = (e: StorageEvent) => {
    if (e.key === STORAGE_KEY) onChange()
  }
  window.addEventListener('storage', onStorage)
  window.addEventListener(CHANGE_EVENT, onChange)
  return () => {
    window.removeEventListener('storage', onStorage)
    window.removeEventListener(CHANGE_EVENT, onChange)
  }
}

export function useAdvancedMode(): [boolean, (next: boolean) => void] {
  const enabled = useSyncExternalStore(subscribe, getSnapshot, () => false)
  const set = (next: boolean) => {
    window.localStorage.setItem(STORAGE_KEY, next ? '1' : '0')
    window.dispatchEvent(new Event(CHANGE_EVENT))
  }
  return [enabled, set]
}
