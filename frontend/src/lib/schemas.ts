import { z } from 'zod'

export const loginSchema = z.object({
  username: z.string().min(1, 'Username is required'),
  password: z.string().min(1, 'Password is required'),
})

export type LoginForm = z.infer<typeof loginSchema>

export const registerSchema = z.object({
  username: z
    .string()
    .min(3, 'Username must be at least 3 characters')
    .max(32, 'Username must be at most 32 characters')
    .regex(/^[a-zA-Z0-9_-]+$/, 'Only letters, numbers, underscores, and hyphens'),
  email: z.string().email('Invalid email address'),
  password: z.string().min(8, 'Password must be at least 8 characters'),
})

export type RegisterForm = z.infer<typeof registerSchema>

export const transactionSchema = z
  .object({
    date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/, 'Date must be YYYY-MM-DD'),
    payee: z.string().min(1, 'Payee is required'),
    debit_account: z.string().min(1, 'Required'),
    credit_account: z.string().min(1, 'Required'),
    amount: z
      .string()
      .min(1, 'Amount is required')
      .regex(/^\d+(\.\d{1,2})?$/, 'Must be a positive decimal'),
  })
  .refine((data) => data.debit_account !== data.credit_account, {
    message: 'Debit and credit accounts must be different',
    path: ['credit_account'],
  })

export type TransactionForm = z.infer<typeof transactionSchema>
