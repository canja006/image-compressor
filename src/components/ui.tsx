// Small, consistent UI primitives shared across the app. Kept deliberately flat: 1px borders,
// near-zero shadows, crisp 6-12px radii, motion only on transform/opacity/color.

import { useEffect, useRef, useState } from 'react'
import type { ButtonHTMLAttributes, ReactNode } from 'react'
import { cx } from '../lib/cx'

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant
}

const BUTTON_VARIANTS: Record<ButtonVariant, string> = {
  primary: 'bg-accent text-accent-fg hover:bg-accent/90 px-4 py-2.5 shadow-subtle',
  secondary: 'bg-surface text-ink border border-line hover:border-line-strong hover:bg-sunken px-3.5 py-2',
  ghost: 'text-muted hover:text-ink hover:bg-sunken px-2.5 py-1.5',
  danger: 'text-bad border border-transparent hover:bg-bad-bg px-3.5 py-2',
}

export function Button({ variant = 'secondary', className, type, ...rest }: ButtonProps) {
  return (
    <button
      type={type ?? 'button'}
      className={cx(
        'inline-flex select-none items-center justify-center gap-2 rounded-md text-sm font-medium',
        'transition-[transform,background-color,border-color,color] duration-150 ease-out',
        'active:scale-[0.98] disabled:pointer-events-none disabled:opacity-40',
        BUTTON_VARIANTS[variant],
        className,
      )}
      {...rest}
    />
  )
}

interface SegmentedOption<T extends string> {
  value: T
  label: ReactNode
  title?: string
}

interface SegmentedProps<T extends string> {
  options: ReadonlyArray<SegmentedOption<T>>
  value: T
  onChange: (value: T) => void
  ariaLabel?: string
  className?: string
  stretch?: boolean
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  ariaLabel,
  className,
  stretch,
}: SegmentedProps<T>) {
  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      className={cx(
        'items-center gap-0.5 rounded-md border border-line bg-sunken p-0.5',
        stretch ? 'flex w-full' : 'inline-flex',
        className,
      )}
    >
      {options.map((opt) => {
        const active = opt.value === value
        return (
          <button
            key={opt.value}
            type="button"
            role="radio"
            aria-checked={active}
            title={opt.title}
            onClick={() => onChange(opt.value)}
            className={cx(
              'rounded-[5px] px-2.5 py-1.5 text-xs font-medium transition-colors duration-150',
              stretch && 'flex-1',
              active ? 'bg-surface text-ink shadow-subtle' : 'text-muted hover:text-ink',
            )}
          >
            {opt.label}
          </button>
        )
      })}
    </div>
  )
}

interface ToggleProps {
  checked: boolean
  onChange: (checked: boolean) => void
  ariaLabel?: string
  id?: string
}

export function Toggle({ checked, onChange, ariaLabel, id }: ToggleProps) {
  return (
    <button
      id={id}
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      onClick={() => onChange(!checked)}
      className={cx(
        'relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors duration-150',
        checked ? 'border-accent bg-accent' : 'border-line bg-sunken',
      )}
    >
      <span
        className={cx(
          'inline-block h-3.5 w-3.5 rounded-full bg-surface transition-transform duration-150 ease-out',
          checked ? 'translate-x-[18px]' : 'translate-x-[3px]',
        )}
      />
    </button>
  )
}

interface NumberFieldProps {
  value: number
  onChange: (value: number) => void
  min?: number
  max?: number
  step?: number
  suffix?: ReactNode
  disabled?: boolean
  ariaLabel?: string
  className?: string
}

export function NumberField({
  value,
  onChange,
  min,
  max,
  step,
  suffix,
  disabled,
  ariaLabel,
  className,
}: NumberFieldProps) {
  // A local draft so the field can be emptied or hold an intermediate value while typing. Valid
  // numbers propagate live; clamping to [min, max] (and reverting an empty box) happens only on blur
  // — coercing on every keystroke is what made an empty field snap straight back to the minimum.
  const [text, setText] = useState(() => (Number.isFinite(value) ? String(value) : ''))
  const textRef = useRef(text)
  textRef.current = text

  // Adopt the value when it changes from outside (e.g. a preset) — but never clobber the draft the
  // user is actively typing (their text already parses to the current value).
  useEffect(() => {
    if (Number.parseFloat(textRef.current) !== value) {
      setText(Number.isFinite(value) ? String(value) : '')
    }
  }, [value])

  const commit = () => {
    const next = Number.parseFloat(text)
    if (Number.isNaN(next)) {
      setText(Number.isFinite(value) ? String(value) : '')
      return
    }
    let clamped = next
    if (min !== undefined) clamped = Math.max(min, clamped)
    if (max !== undefined) clamped = Math.min(max, clamped)
    if (clamped !== value) onChange(clamped)
    setText(String(clamped))
  }

  return (
    <div
      className={cx(
        'flex items-center rounded-md border border-line bg-surface transition-colors duration-150',
        'focus-within:border-line-strong',
        disabled && 'opacity-50',
        className,
      )}
    >
      <input
        type="number"
        inputMode="numeric"
        aria-label={ariaLabel}
        value={text}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        onChange={(e) => {
          setText(e.target.value)
          const next = Number.parseFloat(e.target.value)
          if (!Number.isNaN(next)) onChange(next)
        }}
        onBlur={commit}
        className="w-full bg-transparent px-3 py-2 text-sm tabular-nums text-ink outline-none [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none"
      />
      {suffix != null && (
        <span className="px-2.5 text-xs font-medium text-faint">{suffix}</span>
      )}
    </div>
  )
}

export type BadgeTone = 'good' | 'warn' | 'bad' | 'info' | 'neutral'

const TONE_CLASS: Record<BadgeTone, string> = {
  good: 'bg-good-bg text-good',
  warn: 'bg-warn-bg text-warn',
  bad: 'bg-bad-bg text-bad',
  info: 'bg-info-bg text-info',
  neutral: 'bg-sunken text-muted',
}

export function Badge({
  tone = 'neutral',
  children,
  className,
}: {
  tone?: BadgeTone
  children: ReactNode
  className?: string
}) {
  return (
    <span
      className={cx(
        'inline-flex items-center gap-1 whitespace-nowrap rounded-full px-2 py-0.5 text-[10.5px] font-semibold uppercase tracking-[0.04em]',
        TONE_CLASS[tone],
        className,
      )}
    >
      {children}
    </span>
  )
}

export function Field({
  label,
  hint,
  htmlFor,
  children,
  action,
}: {
  label: string
  hint?: ReactNode
  htmlFor?: string
  children: ReactNode
  action?: ReactNode
}) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <label htmlFor={htmlFor} className="block text-xs font-medium text-muted">
          {label}
        </label>
        {action}
      </div>
      {children}
      {hint != null && <p className="text-[11px] leading-relaxed text-faint">{hint}</p>}
    </div>
  )
}

export function Panel({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <section className={cx('rounded-xl border border-line bg-surface', className)}>{children}</section>
  )
}
