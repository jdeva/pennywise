import { useState, useEffect } from 'react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { useAuth } from '@/context/auth-context'
import { usersApi } from '@/lib/api/users'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { AccountTree } from './account-tree'
import { WorkspacesTab } from './workspaces'
import { UsersTab } from './users-tab'

const profileSchema = z.object({
  username: z.string().min(3).max(32).regex(/^[a-zA-Z0-9_-]+$/),
  email: z.string().email(),
})

const passwordSchema = z.object({
  current_password: z.string().min(1, 'Required'),
  new_password: z.string().min(8, 'Min 8 characters'),
})

export function SettingsPage() {
  const { user, logout } = useAuth()
  const [profileMsg, setProfileMsg] = useState<string | null>(null)
  const [profileError, setProfileError] = useState<string | null>(null)
  const [pwMsg, setPwMsg] = useState<string | null>(null)
  const [pwError, setPwError] = useState<string | null>(null)
  const [deactivatePassword, setDeactivatePassword] = useState('')
  const [deactivateError, setDeactivateError] = useState<string | null>(null)

  const profileForm = useForm<z.infer<typeof profileSchema>>({
    resolver: zodResolver(profileSchema),
    defaultValues: { username: user?.username ?? '', email: user?.email ?? '' },
  })

  useEffect(() => {
    if (user) profileForm.reset({ username: user.username, email: user.email })
  }, [user])

  const pwForm = useForm<z.infer<typeof passwordSchema>>({
    resolver: zodResolver(passwordSchema),
  })

  const onProfileSubmit = async (data: z.infer<typeof profileSchema>) => {
    setProfileMsg(null); setProfileError(null)
    try {
      await usersApi.updateProfile(data)
      setProfileMsg('Profile updated.')
    } catch (err: any) {
      setProfileError(err?.response?.data?.error || 'Failed to update profile')
    }
  }

  const onPasswordSubmit = async (data: z.infer<typeof passwordSchema>) => {
    setPwMsg(null); setPwError(null)
    try {
      await usersApi.changePassword(data.current_password, data.new_password)
      setPwMsg('Password changed successfully.')
      pwForm.reset()
    } catch (err: any) {
      setPwError(err?.response?.data?.error || 'Failed to change password')
    }
  }

  const onDeactivate = async () => {
    if (!confirm('Are you sure? This cannot be undone.')) return
    setDeactivateError(null)
    try {
      await usersApi.deactivate(deactivatePassword)
      await logout()
    } catch (err: any) {
      setDeactivateError(err?.response?.data?.error || 'Failed to deactivate')
    }
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Settings</h1>

      <Tabs defaultValue="profile">
        <TabsList>
          <TabsTrigger value="profile">Profile</TabsTrigger>
          <TabsTrigger value="accounts">Accounts</TabsTrigger>
          <TabsTrigger value="workspaces">Workspaces</TabsTrigger>
          {user?.is_admin && <TabsTrigger value="users">Users</TabsTrigger>}
        </TabsList>

        {/* Profile Tab */}
        <TabsContent value="profile">
          <div className="space-y-6 max-w-lg">
            <Card>
              <CardHeader><CardTitle>Profile</CardTitle></CardHeader>
              <form onSubmit={profileForm.handleSubmit(onProfileSubmit)}>
                <CardContent className="space-y-4">
                  {profileMsg && <div className="rounded-md bg-green-500/10 p-2 text-sm text-green-700">{profileMsg}</div>}
                  {profileError && <div role="alert" className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">{profileError}</div>}
                  <div className="space-y-2">
                    <Label htmlFor="username">Username</Label>
                    <Input id="username" {...profileForm.register('username')} />
                    {profileForm.formState.errors.username && <p className="text-sm text-destructive">{profileForm.formState.errors.username.message}</p>}
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="email">Email</Label>
                    <Input id="email" type="email" {...profileForm.register('email')} />
                    {profileForm.formState.errors.email && <p className="text-sm text-destructive">{profileForm.formState.errors.email.message}</p>}
                  </div>
                  <Button type="submit" disabled={profileForm.formState.isSubmitting}>Update Profile</Button>
                </CardContent>
              </form>
            </Card>

            <Card>
              <CardHeader><CardTitle>Change Password</CardTitle></CardHeader>
              <form onSubmit={pwForm.handleSubmit(onPasswordSubmit)}>
                <CardContent className="space-y-4">
                  {pwMsg && <div className="rounded-md bg-green-500/10 p-2 text-sm text-green-700">{pwMsg}</div>}
                  {pwError && <div role="alert" className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">{pwError}</div>}
                  <div className="space-y-2">
                    <Label htmlFor="current_password">Current Password</Label>
                    <Input id="current_password" type="password" {...pwForm.register('current_password')} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="new_password">New Password</Label>
                    <Input id="new_password" type="password" {...pwForm.register('new_password')} />
                    {pwForm.formState.errors.new_password && <p className="text-sm text-destructive">{pwForm.formState.errors.new_password.message}</p>}
                  </div>
                  <Button type="submit" disabled={pwForm.formState.isSubmitting}>Change Password</Button>
                </CardContent>
              </form>
            </Card>

            <Card className="border-destructive">
              <CardHeader><CardTitle className="text-destructive">Deactivate Account</CardTitle></CardHeader>
              <CardContent className="space-y-4">
                {deactivateError && <div role="alert" className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">{deactivateError}</div>}
                <div className="space-y-2">
                  <Label htmlFor="deactivate_password">Confirm Password</Label>
                  <Input id="deactivate_password" type="password" value={deactivatePassword} onChange={(e) => setDeactivatePassword(e.target.value)} />
                </div>
                <Button variant="destructive" onClick={onDeactivate} disabled={!deactivatePassword}>
                  Deactivate Account
                </Button>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Accounts Tab */}
        <TabsContent value="accounts">
          <AccountTree />
        </TabsContent>

        {/* Workspaces Tab */}
        <TabsContent value="workspaces">
          <WorkspacesTab />
        </TabsContent>

        {/* Users Tab (admin only) */}
        {user?.is_admin && (
          <TabsContent value="users">
            <UsersTab />
          </TabsContent>
        )}
      </Tabs>
    </div>
  )
}
