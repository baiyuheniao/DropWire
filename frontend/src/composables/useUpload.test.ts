import { describe, expect, it } from 'vitest'
import { deriveUploadId, getRelativePath, simpleHash } from './useUpload'

function makeFile(name: string, content = 'hello', lastModified = 1700000000000): File {
  return new File([content], name, { lastModified })
}

describe('simpleHash', () => {
  it('is deterministic for the same input', () => {
    expect(simpleHash('a|1|2')).toBe(simpleHash('a|1|2'))
  })

  it('produces an 8-char lowercase hex string', () => {
    expect(simpleHash('anything')).toMatch(/^[0-9a-f]{8}$/)
  })

  it('differs for different inputs', () => {
    expect(simpleHash('a')).not.toBe(simpleHash('b'))
  })
})

describe('deriveUploadId', () => {
  it('is stable for the same file fingerprint without a password', () => {
    const a = makeFile('report.pdf')
    const b = makeFile('report.pdf')
    expect(deriveUploadId(a)).toBe(deriveUploadId(b))
  })

  it('differs when the file fingerprint differs', () => {
    const a = makeFile('report.pdf')
    const b = makeFile('other.pdf')
    expect(deriveUploadId(a)).not.toBe(deriveUploadId(b))
  })

  it('uses a random id (not the file fingerprint) when a password is set', () => {
    // Encrypted uploads produce different ciphertext each time, so resuming
    // by content fingerprint would not work - it must fall back to random ids.
    const file = makeFile('secret.pdf')
    const withoutPassword = deriveUploadId(file)
    const withPassword = deriveUploadId(file, 'hunter2')
    expect(withPassword).not.toBe(withoutPassword)
  })

  it('generates a different random id on every call when a password is set', () => {
    const file = makeFile('secret.pdf')
    const first = deriveUploadId(file, 'hunter2')
    const second = deriveUploadId(file, 'hunter2')
    expect(first).not.toBe(second)
  })
})

describe('getRelativePath', () => {
  it('returns undefined when webkitRelativePath is absent', () => {
    const file = makeFile('a.txt')
    expect(getRelativePath(file)).toBeUndefined()
  })

  it('returns the relative path when present', () => {
    const file = makeFile('a.txt')
    Object.defineProperty(file, 'webkitRelativePath', {
      value: 'folder/a.txt',
    })
    expect(getRelativePath(file)).toBe('folder/a.txt')
  })
})
