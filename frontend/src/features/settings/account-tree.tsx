import { useState, useRef, useEffect } from 'react'
import type { AccountTreeNode, AccountType } from '@/lib/account-tree'
import { useAccountTree } from './use-account-tree'
import { useWorkspace } from '@/context/workspace-context'
import { transactionsApi } from '@/lib/api/transactions'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ChevronRight, ChevronDown, Plus, Trash2 } from 'lucide-react'

const AMOUNT_REGEX = /^\d+(\.\d{1,2})?$/

async function postOpeningBalance(workspaceId: string, accountFullPath: string, amount: string) {
  const today = new Date().toISOString().split('T')[0]
  await transactionsApi.post(workspaceId, {
    date: today,
    payee: 'Opening Balance',
    debit_account: accountFullPath,
    credit_account: 'Equity:Opening Balances',
    amount,
  })
}

function shouldPromptBalance(accountType: AccountType): boolean {
  return accountType === 'assets' || accountType === 'liabilities'
}

interface TreeNodeProps {
  node: AccountTreeNode
  accountType: AccountType
  depth: number
  workspaceId: string
  onAdd: (parentPath: string, leafName: string) => Promise<string>
  onDelete: (fullPath: string) => Promise<void>
}

function TreeNode({ node, accountType, depth, workspaceId, onAdd, onDelete }: TreeNodeProps) {
  const [expanded, setExpanded] = useState(false)
  const [showInput, setShowInput] = useState(false)
  const [inputValue, setInputValue] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  // Opening balance state
  const [showBalanceInput, setShowBalanceInput] = useState(false)
  const [balanceValue, setBalanceValue] = useState('')
  const [balanceError, setBalanceError] = useState<string | null>(null)
  const [isPostingBalance, setIsPostingBalance] = useState(false)
  const [createdFullPath, setCreatedFullPath] = useState<string | null>(null)
  const balanceInputRef = useRef<HTMLInputElement>(null)

  const isLeaf = node.children.length === 0
  const hasChildren = node.children.length > 0

  useEffect(() => {
    if (showInput && inputRef.current) {
      inputRef.current.focus()
    }
  }, [showInput])

  useEffect(() => {
    if (showBalanceInput && balanceInputRef.current) {
      balanceInputRef.current.focus()
    }
  }, [showBalanceInput])

  const handleAdd = async () => {
    const trimmed = inputValue.trim()
    if (!trimmed) {
      setError('Name cannot be empty')
      return
    }
    setIsSubmitting(true)
    setError(null)
    try {
      const fullPath = await onAdd(node.fullPath, trimmed)
      setInputValue('')
      setShowInput(false)
      setExpanded(true)
      if (shouldPromptBalance(accountType)) {
        setCreatedFullPath(fullPath)
        setShowBalanceInput(true)
      }
    } catch (err: any) {
      if (err?.response?.status === 409) {
        setError('An account with this name already exists')
      } else {
        setError(err?.message || 'Failed to add account')
      }
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      handleAdd()
    } else if (e.key === 'Escape') {
      setShowInput(false)
      setInputValue('')
      setError(null)
    }
  }

  const dismissBalance = () => {
    setShowBalanceInput(false)
    setBalanceValue('')
    setBalanceError(null)
    setCreatedFullPath(null)
  }

  const handleBalanceSubmit = async () => {
    const trimmed = balanceValue.trim()
    if (!trimmed) {
      setBalanceError('Amount is required')
      return
    }
    if (!AMOUNT_REGEX.test(trimmed)) {
      setBalanceError('Must be a valid amount')
      return
    }
    setIsPostingBalance(true)
    setBalanceError(null)
    try {
      await postOpeningBalance(workspaceId, createdFullPath!, trimmed)
      dismissBalance()
    } catch (err: any) {
      setBalanceError(err?.response?.data?.error || err?.message || 'Failed to post opening balance')
    } finally {
      setIsPostingBalance(false)
    }
  }

  const handleBalanceKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      handleBalanceSubmit()
    } else if (e.key === 'Escape') {
      dismissBalance()
    }
  }

  return (
    <div>
      <div
        className="flex items-center gap-1 rounded px-1 py-0.5 hover:bg-muted group"
        style={{ paddingLeft: `${depth * 20}px` }}
      >
        {hasChildren ? (
          <button
            onClick={() => setExpanded(!expanded)}
            className="p-0.5 rounded hover:bg-muted-foreground/10"
            aria-label={expanded ? 'Collapse' : 'Expand'}
          >
            {expanded ? (
              <ChevronDown className="h-4 w-4" />
            ) : (
              <ChevronRight className="h-4 w-4" />
            )}
          </button>
        ) : (
          <span className="w-5" />
        )}

        <span className="text-sm flex-1">{node.name}</span>

        <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => {
              setShowInput(true)
              setExpanded(true)
            }}
            aria-label={`Add child to ${node.name}`}
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>

          {isLeaf && (
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 text-destructive hover:text-destructive"
              onClick={() => onDelete(node.fullPath)}
              aria-label={`Delete ${node.name}`}
            >
              <Trash2 className="h-3.5 w-3.5" />
            </Button>
          )}
        </div>
      </div>

      {expanded && hasChildren && (
        <div>
          {node.children.map((child) => (
            <TreeNode
              key={child.fullPath}
              node={child}
              accountType={accountType}
              depth={depth + 1}
              workspaceId={workspaceId}
              onAdd={onAdd}
              onDelete={onDelete}
            />
          ))}
        </div>
      )}

      {showInput && (
        <div
          className="flex items-center gap-2 py-1"
          style={{ paddingLeft: `${(depth + 1) * 20 + 20}px` }}
        >
          <Input
            ref={inputRef}
            value={inputValue}
            onChange={(e) => {
              setInputValue(e.target.value)
              if (error) setError(null)
            }}
            onKeyDown={handleKeyDown}
            placeholder="Account name…"
            className="h-7 text-sm flex-1 max-w-[200px]"
            disabled={isSubmitting}
          />
          <Button
            size="sm"
            className="h-7 text-xs"
            onClick={handleAdd}
            disabled={isSubmitting}
          >
            Add
          </Button>
          {error && (
            <span role="alert" className="text-xs text-destructive">
              {error}
            </span>
          )}
        </div>
      )}

      {showBalanceInput && (
        <div
          className="flex items-center gap-2 py-1"
          style={{ paddingLeft: `${(depth + 1) * 20 + 20}px` }}
        >
          <Input
            ref={balanceInputRef}
            value={balanceValue}
            onChange={(e) => {
              setBalanceValue(e.target.value)
              if (balanceError) setBalanceError(null)
            }}
            onKeyDown={handleBalanceKeyDown}
            placeholder="Opening balance…"
            className="h-7 text-sm flex-1 max-w-[200px]"
            disabled={isPostingBalance}
          />
          <Button
            size="sm"
            className="h-7 text-xs"
            onClick={handleBalanceSubmit}
            disabled={isPostingBalance}
          >
            Set
          </Button>
          {balanceError && (
            <span role="alert" className="text-xs text-destructive">
              {balanceError}
            </span>
          )}
        </div>
      )}
    </div>
  )
}


interface RootSectionProps {
  label: string
  accountType: AccountType
  children: AccountTreeNode[]
  workspaceId: string
  onAdd: (parentPath: string, leafName: string, accountType: AccountType) => Promise<string>
  onDelete: (fullPath: string, accountType: AccountType) => Promise<void>
}

function RootSection({ label, accountType, children, workspaceId, onAdd, onDelete }: RootSectionProps) {
  const [expanded, setExpanded] = useState(false)
  const [showInput, setShowInput] = useState(false)
  const [inputValue, setInputValue] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  // Opening balance state
  const [showBalanceInput, setShowBalanceInput] = useState(false)
  const [balanceValue, setBalanceValue] = useState('')
  const [balanceError, setBalanceError] = useState<string | null>(null)
  const [isPostingBalance, setIsPostingBalance] = useState(false)
  const [createdFullPath, setCreatedFullPath] = useState<string | null>(null)
  const balanceInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (showInput && inputRef.current) {
      inputRef.current.focus()
    }
  }, [showInput])

  useEffect(() => {
    if (showBalanceInput && balanceInputRef.current) {
      balanceInputRef.current.focus()
    }
  }, [showBalanceInput])

  const handleNodeAdd = async (parentPath: string, leafName: string): Promise<string> => {
    return await onAdd(parentPath, leafName, accountType)
  }

  const handleNodeDelete = async (fullPath: string) => {
    await onDelete(fullPath, accountType)
  }

  const handleRootAdd = async () => {
    const trimmed = inputValue.trim()
    if (!trimmed) {
      setError('Name cannot be empty')
      return
    }
    setIsSubmitting(true)
    setError(null)
    try {
      const fullPath = await onAdd('', trimmed, accountType)
      setInputValue('')
      setShowInput(false)
      setExpanded(true)
      if (shouldPromptBalance(accountType)) {
        setCreatedFullPath(fullPath)
        setShowBalanceInput(true)
      }
    } catch (err: any) {
      if (err?.response?.status === 409) {
        setError('An account with this name already exists')
      } else {
        setError(err?.message || 'Failed to add account')
      }
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      handleRootAdd()
    } else if (e.key === 'Escape') {
      setShowInput(false)
      setInputValue('')
      setError(null)
    }
  }

  const dismissBalance = () => {
    setShowBalanceInput(false)
    setBalanceValue('')
    setBalanceError(null)
    setCreatedFullPath(null)
  }

  const handleBalanceSubmit = async () => {
    const trimmed = balanceValue.trim()
    if (!trimmed) {
      setBalanceError('Amount is required')
      return
    }
    if (!AMOUNT_REGEX.test(trimmed)) {
      setBalanceError('Must be a valid amount')
      return
    }
    setIsPostingBalance(true)
    setBalanceError(null)
    try {
      await postOpeningBalance(workspaceId, createdFullPath!, trimmed)
      dismissBalance()
    } catch (err: any) {
      setBalanceError(err?.response?.data?.error || err?.message || 'Failed to post opening balance')
    } finally {
      setIsPostingBalance(false)
    }
  }

  const handleBalanceKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      handleBalanceSubmit()
    } else if (e.key === 'Escape') {
      dismissBalance()
    }
  }

  return (
    <div className="mb-2">
      <div className="flex items-center gap-1 rounded px-1 py-1 hover:bg-muted group">
        <button
          onClick={() => setExpanded(!expanded)}
          className="p-0.5 rounded hover:bg-muted-foreground/10"
          aria-label={expanded ? `Collapse ${label}` : `Expand ${label}`}
        >
          {expanded ? (
            <ChevronDown className="h-4 w-4" />
          ) : (
            <ChevronRight className="h-4 w-4" />
          )}
        </button>
        <span className="text-sm font-semibold flex-1">{label}</span>
        <div className="opacity-0 group-hover:opacity-100 transition-opacity">
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => {
              setShowInput(true)
              setExpanded(true)
            }}
            aria-label={`Add account to ${label}`}
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {expanded && (
        <div>
          {children.map((child) => (
            <TreeNode
              key={child.fullPath}
              node={child}
              accountType={accountType}
              depth={1}
              workspaceId={workspaceId}
              onAdd={handleNodeAdd}
              onDelete={handleNodeDelete}
            />
          ))}
          {children.length === 0 && !showInput && (
            <p
              className="text-xs text-muted-foreground py-1"
              style={{ paddingLeft: '40px' }}
            >
              No accounts
            </p>
          )}
        </div>
      )}

      {showInput && (
        <div className="flex items-center gap-2 py-1" style={{ paddingLeft: '40px' }}>
          <Input
            ref={inputRef}
            value={inputValue}
            onChange={(e) => {
              setInputValue(e.target.value)
              if (error) setError(null)
            }}
            onKeyDown={handleKeyDown}
            placeholder="Account name…"
            className="h-7 text-sm flex-1 max-w-[200px]"
            disabled={isSubmitting}
          />
          <Button
            size="sm"
            className="h-7 text-xs"
            onClick={handleRootAdd}
            disabled={isSubmitting}
          >
            Add
          </Button>
          {error && (
            <span role="alert" className="text-xs text-destructive">
              {error}
            </span>
          )}
        </div>
      )}

      {showBalanceInput && (
        <div className="flex items-center gap-2 py-1" style={{ paddingLeft: '40px' }}>
          <Input
            ref={balanceInputRef}
            value={balanceValue}
            onChange={(e) => {
              setBalanceValue(e.target.value)
              if (balanceError) setBalanceError(null)
            }}
            onKeyDown={handleBalanceKeyDown}
            placeholder="Opening balance…"
            className="h-7 text-sm flex-1 max-w-[200px]"
            disabled={isPostingBalance}
          />
          <Button
            size="sm"
            className="h-7 text-xs"
            onClick={handleBalanceSubmit}
            disabled={isPostingBalance}
          >
            Set
          </Button>
          {balanceError && (
            <span role="alert" className="text-xs text-destructive">
              {balanceError}
            </span>
          )}
        </div>
      )}
    </div>
  )
}

export function AccountTree() {
  const { activeWorkspace } = useWorkspace()
  const { trees, isLoading, addAccount, deleteAccount } = useAccountTree()

  if (!activeWorkspace) {
    return (
      <div className="max-w-2xl">
        <Card>
          <CardContent className="py-6">
            <p className="text-sm text-muted-foreground">
              Please select a workspace to manage accounts.
            </p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="max-w-2xl">
      <Card>
        <CardHeader>
          <CardTitle>Chart of Accounts</CardTitle>
          <p className="text-sm text-muted-foreground">
            Manage your ledger account hierarchy across five account types.
          </p>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-2">
              {[...Array(5)].map((_, i) => (
                <div key={i} className="h-8 animate-pulse rounded bg-muted" />
              ))}
            </div>
          ) : (
            <div>
              {trees.map((tree) => (
                <RootSection
                  key={tree.type}
                  label={tree.label}
                  accountType={tree.type}
                  children={tree.children}
                  workspaceId={activeWorkspace.id}
                  onAdd={addAccount}
                  onDelete={deleteAccount}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
