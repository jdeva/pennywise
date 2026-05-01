import { useEffect, useMemo, useRef, useState } from 'react'
import { EditorState } from '@codemirror/state'
import { EditorView, keymap, lineNumbers, highlightActiveLine } from '@codemirror/view'
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands'
import { ledgerFilesApi, type LedgerFileEntry } from '@/lib/api/ledger-files'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { useAdvancedMode } from '@/lib/use-advanced-mode'
import { cn } from '@/lib/utils'

type SaveState = 'idle' | 'saving' | 'saved' | 'error'
type ValidateResult = { ok: boolean; output: string }

function errorMessage(e: unknown, fallback: string): string {
  const msg = (e as { response?: { data?: { error?: string } } })?.response?.data?.error
  return typeof msg === 'string' ? msg : fallback
}

export function LedgerFilesPage() {
  const [advanced] = useAdvancedMode()
  const [files, setFiles] = useState<LedgerFileEntry[]>([])
  const [listLoading, setListLoading] = useState(true)
  const [listError, setListError] = useState<string | null>(null)
  const [selected, setSelected] = useState<string | null>(null)
  const [contentLoading, setContentLoading] = useState(false)
  const [contentError, setContentError] = useState<string | null>(null)
  const [content, setContent] = useState('')
  const [dirty, setDirty] = useState(false)
  const [saveState, setSaveState] = useState<SaveState>('idle')
  const [saveMessage, setSaveMessage] = useState<string | null>(null)
  const [validateResult, setValidateResult] = useState<ValidateResult | null>(null)
  const [validating, setValidating] = useState(false)

  // Live refs let the CodeMirror save-keybinding read current state without rebuilding the editor.
  const contentRef = useRef('')
  contentRef.current = content
  const selectedRef = useRef<string | null>(null)
  selectedRef.current = selected
  const dirtyRef = useRef(false)
  dirtyRef.current = dirty

  const viewRef = useRef<EditorView | null>(null)
  const [editorHost, setEditorHost] = useState<HTMLDivElement | null>(null)

  const onSave = async () => {
    const path = selectedRef.current
    if (!path || !dirtyRef.current) return
    setSaveState('saving')
    setSaveMessage(null)
    try {
      await ledgerFilesApi.write(path, contentRef.current)
      setDirty(false)
      setSaveState('saved')
      setSaveMessage('Saved.')
    } catch (e) {
      setSaveState('error')
      setSaveMessage(errorMessage(e, 'Failed to save'))
    }
  }

  const onValidate = async () => {
    setValidating(true)
    setValidateResult(null)
    try {
      setValidateResult(await ledgerFilesApi.validate(contentRef.current))
    } catch (e) {
      setValidateResult({ ok: false, output: errorMessage(e, 'Failed to validate') })
    } finally {
      setValidating(false)
    }
  }

  // Load file list once advanced mode is active.
  useEffect(() => {
    if (!advanced) return
    let cancelled = false
    setListLoading(true)
    setListError(null)
    ledgerFilesApi
      .list()
      .then((rows) => {
        if (cancelled) return
        setFiles(rows)
        setSelected((prev) => prev ?? rows[0]?.path ?? null)
      })
      .catch((e) => !cancelled && setListError(errorMessage(e, 'Failed to load files')))
      .finally(() => !cancelled && setListLoading(false))
    return () => {
      cancelled = true
    }
  }, [advanced])

  // Load file content when selection changes.
  useEffect(() => {
    if (!selected) return
    let cancelled = false
    setContentLoading(true)
    setContentError(null)
    setValidateResult(null)
    setSaveMessage(null)
    setSaveState('idle')
    ledgerFilesApi
      .read(selected)
      .then((res) => {
        if (cancelled) return
        setContent(res.content)
        setDirty(false)
      })
      .catch((e) => !cancelled && setContentError(errorMessage(e, 'Failed to read file')))
      .finally(() => !cancelled && setContentLoading(false))
    return () => {
      cancelled = true
    }
  }, [selected])

  // Create (and recreate, on file switch) the CodeMirror view.
  useEffect(() => {
    if (!editorHost) return
    viewRef.current?.destroy()
    const view = new EditorView({
      state: EditorState.create({
        doc: content,
        extensions: [
          lineNumbers(),
          highlightActiveLine(),
          history(),
          keymap.of([
            {
              key: 'Mod-s',
              preventDefault: true,
              run: () => {
                void onSave()
                return true
              },
            },
            ...defaultKeymap,
            ...historyKeymap,
          ]),
          EditorView.lineWrapping,
          EditorView.theme({
            '&': { height: '100%', fontSize: '13px' },
            '.cm-scroller': { fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace' },
          }),
          EditorView.updateListener.of((upd) => {
            if (!upd.docChanged) return
            const next = upd.state.doc.toString()
            if (next !== contentRef.current) {
              setContent(next)
              setDirty(true)
              setSaveState('idle')
            }
          }),
        ],
      }),
      parent: editorHost,
    })
    viewRef.current = view
    return () => {
      view.destroy()
      viewRef.current = null
    }
    // `content` is the initial doc; later updates flow via updateListener. Recreate only on
    // host remount or file switch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected, editorHost])

  const grouped = useMemo(() => {
    const byWs = new Map<string, { label: string; items: LedgerFileEntry[] }>()
    const loose: LedgerFileEntry[] = []
    for (const f of files) {
      if (f.workspace_id && f.workspace_name) {
        let g = byWs.get(f.workspace_id)
        if (!g) {
          g = { label: f.workspace_name, items: [] }
          byWs.set(f.workspace_id, g)
        }
        g.items.push(f)
      } else {
        loose.push(f)
      }
    }
    return { byWs: Array.from(byWs.entries()), loose }
  }, [files])

  if (!advanced) {
    return (
      <div className="space-y-4">
        <h1 className="text-2xl font-bold">Ledger files</h1>
        <Card>
          <CardContent className="py-6">
            <p className="text-sm text-muted-foreground">
              This feature is off. Enable <span className="font-medium">Ledger file editor</span> under
              Settings → Advanced to use it.
            </p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-2">
        <h1 className="text-2xl font-bold">Ledger files</h1>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={onValidate} disabled={validating || !selected}>
            {validating ? 'Validating…' : 'Validate'}
          </Button>
          <Button onClick={onSave} disabled={!dirty || saveState === 'saving' || !selected}>
            {saveState === 'saving' ? 'Saving…' : dirty ? 'Save' : 'Saved'}
          </Button>
        </div>
      </div>

      {saveMessage && (
        <div
          role="alert"
          className={cn(
            'rounded-md p-2 text-sm',
            saveState === 'error'
              ? 'bg-destructive/10 text-destructive'
              : 'bg-green-500/10 text-green-700',
          )}
        >
          {saveMessage}
        </div>
      )}

      {validateResult && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">
              {validateResult.ok ? 'Ledger validation passed' : 'Ledger validation failed'}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <pre
              className={cn(
                'max-h-60 overflow-auto whitespace-pre-wrap rounded-md p-3 text-xs',
                validateResult.ok ? 'bg-green-500/10' : 'bg-destructive/10 text-destructive',
              )}
            >
              {validateResult.output || (validateResult.ok ? 'OK' : '(no output)')}
            </pre>
          </CardContent>
        </Card>
      )}

      <div className="grid grid-cols-1 gap-4 md:grid-cols-[260px_1fr]">
        <Card className="md:max-h-[70vh] md:overflow-auto">
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Files</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {listLoading && <p className="text-sm text-muted-foreground">Loading…</p>}
            {listError && (
              <p role="alert" className="text-sm text-destructive">
                {listError}
              </p>
            )}
            {!listLoading && !listError && files.length === 0 && (
              <p className="text-sm text-muted-foreground">No ledger files.</p>
            )}

            {grouped.loose.length > 0 && (
              <FileGroup label="Master" items={grouped.loose} selected={selected} onSelect={setSelected} />
            )}
            {grouped.byWs.map(([wsId, g]) => (
              <FileGroup
                key={wsId}
                label={g.label}
                items={g.items}
                selected={selected}
                onSelect={setSelected}
              />
            ))}
          </CardContent>
        </Card>

        <Card className="overflow-hidden">
          <CardContent className="p-0">
            {!selected ? (
              <div className="p-6 text-sm text-muted-foreground">Select a file to edit.</div>
            ) : contentLoading ? (
              <div className="p-6 text-sm text-muted-foreground">Loading…</div>
            ) : contentError ? (
              <div className="p-6 text-sm text-destructive" role="alert">
                {contentError}
              </div>
            ) : (
              <div
                ref={setEditorHost}
                data-testid="ledger-editor"
                className="h-[60vh] md:h-[70vh]"
              />
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  )
}

function FileGroup({
  label,
  items,
  selected,
  onSelect,
}: {
  label: string
  items: LedgerFileEntry[]
  selected: string | null
  onSelect: (path: string) => void
}) {
  return (
    <div className="space-y-1">
      <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{label}</div>
      {items.map((f) => (
        <button
          key={f.path}
          onClick={() => onSelect(f.path)}
          className={cn(
            'flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm',
            selected === f.path ? 'bg-primary/10 font-medium text-primary' : 'hover:bg-accent',
          )}
        >
          <span className="truncate font-mono text-xs">{f.label}</span>
          <span className="shrink-0 text-[10px] text-muted-foreground">{formatBytes(f.bytes)}</span>
        </button>
      ))}
    </div>
  )
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / (1024 * 1024)).toFixed(1)} MB`
}
