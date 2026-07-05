import { ref } from 'vue'
import axios from 'axios'

export interface User {
  username: string
  nickname: string
  avatar?: string
  token: string
}

const currentUser = ref<User | null>(null)

const USER_KEY = 'dropwire_current_user'

function isValidUser(value: unknown): value is Omit<User, 'token'> {
  if (!value || typeof value !== 'object') return false
  const u = value as Record<string, unknown>
  return (
    typeof u.username === 'string' &&
    u.username.length > 0 &&
    typeof u.nickname === 'string' &&
    (u.avatar === undefined || typeof u.avatar === 'string')
  )
}

export function loadUser(): User | null {
  const raw = localStorage.getItem(USER_KEY)
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw)
    if (!isValidUser(parsed)) {
      localStorage.removeItem(USER_KEY)
      return null
    }
    // Token is intentionally kept in memory only; a stored token without the
    // corresponding server-side session is useless after restart.
    return { ...parsed, token: '' }
  } catch {
    localStorage.removeItem(USER_KEY)
    return null
  }
}

export function setUser(user: User | null) {
  currentUser.value = user
  if (user) {
    const { token, ...safe } = user
    localStorage.setItem(USER_KEY, JSON.stringify(safe))
    axios.defaults.headers.common['Authorization'] = `Bearer ${token}`
  } else {
    // Best-effort: tell the server to drop our session so it doesn't leak in
    // the in-memory session map. Fire-and-forget; ignore failures.
    const auth = axios.defaults.headers.common['Authorization']
    if (auth) {
      axios.post('/auth/logout').catch(() => {})
    }
    localStorage.removeItem(USER_KEY)
    delete axios.defaults.headers.common['Authorization']
  }
}

export function useAuth() {
  return { currentUser, setUser, loadUser }
}
