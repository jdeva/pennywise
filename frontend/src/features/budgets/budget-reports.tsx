import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { budgetsApi } from '@/lib/api/budgets'
import { useWorkspace } from '@/context/workspace-context'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

type ReportTab = 'budget' | 'unbudgeted' | 'forecast'

export function BudgetReports() {
  const { activeWorkspace } = useWorkspace()
  const [tab, setTab] = useState<ReportTab>('budget')
  const [begin, setBegin] = useState('')
  const [end, setEnd] = useState('')
  const [forecastEnd, setForecastEnd] = useState('')

  const { data: budgetReport, isLoading: budgetLoading } = useQuery({
    queryKey: ['budget-report', activeWorkspace?.id, begin, end],
    queryFn: async () => {
      const { data } = await budgetsApi.report(activeWorkspace!.id, begin || undefined, end || undefined)
      return data
    },
    enabled: !!activeWorkspace && tab === 'budget',
  })

  const { data: unbudgetedReport, isLoading: unbudgetedLoading } = useQuery({
    queryKey: ['unbudgeted-report', activeWorkspace?.id, begin, end],
    queryFn: async () => {
      const { data } = await budgetsApi.unbudgeted(activeWorkspace!.id, begin || undefined, end || undefined)
      return data
    },
    enabled: !!activeWorkspace && tab === 'unbudgeted',
  })

  const { data: forecastReport, isLoading: forecastLoading } = useQuery({
    queryKey: ['forecast-report', activeWorkspace?.id, forecastEnd],
    queryFn: async () => {
      const { data } = await budgetsApi.forecast(activeWorkspace!.id, forecastEnd || undefined)
      return data
    },
    enabled: !!activeWorkspace && tab === 'forecast',
  })

  const renderOutput = (output: string | undefined, loading: boolean) => {
    if (loading) return <div className="h-20 animate-pulse rounded bg-muted" />
    if (!output || output.trim() === '') return <p className="text-sm text-muted-foreground">No data.</p>
    return <pre className="whitespace-pre-wrap text-sm font-mono overflow-x-auto">{output}</pre>
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex gap-1">
          {(['budget', 'unbudgeted', 'forecast'] as ReportTab[]).map((t) => (
            <Button key={t} variant={tab === t ? 'default' : 'outline'} size="sm" onClick={() => setTab(t)} className="capitalize">
              {t === 'budget' ? 'Budget vs Actual' : t === 'unbudgeted' ? 'Unbudgeted' : 'Forecast'}
            </Button>
          ))}
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {tab !== 'forecast' && (
          <div className="flex gap-3 items-end">
            <div className="space-y-1">
              <Label>Begin</Label>
              <Input type="date" value={begin} onChange={(e) => setBegin(e.target.value)} />
            </div>
            <div className="space-y-1">
              <Label>End</Label>
              <Input type="date" value={end} onChange={(e) => setEnd(e.target.value)} />
            </div>
          </div>
        )}
        {tab === 'forecast' && (
          <div className="space-y-1 max-w-xs">
            <Label>End Date</Label>
            <Input type="date" value={forecastEnd} onChange={(e) => setForecastEnd(e.target.value)} />
          </div>
        )}
        {tab === 'budget' && renderOutput(budgetReport?.output, budgetLoading)}
        {tab === 'unbudgeted' && renderOutput(unbudgetedReport?.output, unbudgetedLoading)}
        {tab === 'forecast' && renderOutput(forecastReport?.output, forecastLoading)}
      </CardContent>
    </Card>
  )
}
