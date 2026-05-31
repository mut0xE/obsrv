'use client'

import { AlertTriangle, RefreshCw, XCircle } from 'lucide-react'
import { ApiError } from '@/lib/api'

interface ErrorDisplayProps {
  error: Error | ApiError | string
  onRetry?: () => void
  title?: string
}

export function ErrorDisplay({ error, onRetry, title = 'Error' }: ErrorDisplayProps) {
  const errorMessage = typeof error === 'string'
    ? error
    : error.message

  const isApiError = error instanceof ApiError
  const status = isApiError ? error.status : undefined
  const isNetworkError = status === 0

  return (
    <div className="animate-slidefade">
      <div
        className="border px-7 py-6"
        style={{
          background: 'var(--critical-dim)',
          borderColor: 'var(--critical)',
        }}
      >
        <div className="flex items-start gap-4">
          <div className="mt-0.5">
            {isNetworkError ? (
              <XCircle size={18} style={{ color: 'var(--critical)' }} />
            ) : (
              <AlertTriangle size={18} style={{ color: 'var(--critical)' }} />
            )}
          </div>

          <div className="flex-1 min-w-0">
            <div
              className="font-mono text-sm font-medium mb-1"
              style={{ color: 'var(--critical)' }}
            >
              {title}
            </div>

            <div
              className="font-mono text-xs leading-relaxed"
              style={{ color: 'var(--text-secondary)' }}
            >
              {errorMessage}
            </div>

            {status && status !== 0 && (
              <div
                className="font-mono text-[10px] mt-2 uppercase tracking-widest"
                style={{ color: 'var(--text-tertiary)' }}
              >
                Status: {status}
              </div>
            )}

            {isNetworkError && (
              <div
                className="font-mono text-xs mt-3"
                style={{ color: 'var(--text-tertiary)' }}
              >
                • Check if the API server is running
                <br />
                • Verify NEXT_PUBLIC_API_URL is correct
                <br />• Check your network connection
              </div>
            )}
          </div>

          {onRetry && (
            <button
              onClick={onRetry}
              className="px-4 py-2 font-mono text-xs font-medium uppercase tracking-wider flex items-center gap-2 hover:opacity-80 transition-opacity"
              style={{
                background: 'var(--critical)',
                color: 'var(--text-primary)',
              }}
            >
              <RefreshCw size={12} />
              Retry
            </button>
          )}
        </div>
      </div>
    </div>
  )
}

export function EmptyState({ message }: { message: string }) {
  return (
    <div
      className="text-center py-20 px-5 font-mono text-xs uppercase tracking-widest"
      style={{ color: 'var(--text-tertiary)' }}
    >
      {message}
    </div>
  )
}
