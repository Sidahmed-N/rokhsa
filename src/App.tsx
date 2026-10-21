import { NavLink, Navigate, Route, Routes } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  CalendarDays,
  LayoutDashboard,
  Receipt,
  Settings,
  Users,
  Wallet,
} from "lucide-react";

import ClockWarning from "@/components/ClockWarning";
import LanguageSwitcher from "@/components/LanguageSwitcher";
import CalendarPage from "@/features/calendar/CalendarPage";
import CandidateDetailPage from "@/features/candidates/CandidateDetailPage";
import CandidatesPage from "@/features/candidates/CandidatesPage";
import HomePage from "@/features/home/HomePage";
import PrintProofPage from "@/features/print/PrintProofPage";
import ReceiptPage from "@/features/print/ReceiptPage";
import CashFlowPage from "@/features/reports/CashFlowPage";
import ReceivablesPage from "@/features/reports/ReceivablesPage";
import SettingsPage from "@/features/settings/SettingsPage";

/**
 * Application shell.
 *
 * A fixed sidebar rather than a top bar: this is desktop software that one
 * person keeps open all day, and a persistent rail makes the whole product
 * visible at once instead of hiding it behind a row of words. It also leaves
 * the top of every screen free for that screen's own title and controls.
 *
 * Every spacing and alignment utility here is a LOGICAL property
 * (ms-/me-/ps-/pe-/start-/end-/text-start), never a physical one, and the
 * sidebar is simply the first flex child — so the whole layout mirrors for
 * Arabic from the single `dir` attribute on <html>, with the rail moving to
 * the right of the window and the content to its left.
 */
export default function App() {
  const { t } = useTranslation();

  const navItems = [
    { to: "/", labelKey: "nav.home", Icon: LayoutDashboard },
    { to: "/calendar", labelKey: "nav.calendar", Icon: CalendarDays },
    { to: "/candidates", labelKey: "nav.candidates", Icon: Users },
    { to: "/receivables", labelKey: "nav.receivables", Icon: Wallet },
    { to: "/cash-flow", labelKey: "nav.cashFlow", Icon: Receipt },
    { to: "/settings", labelKey: "nav.settings", Icon: Settings },
  ] as const;

  return (
    <div className="flex min-h-screen">
      <aside className="screen-only sticky top-0 flex h-screen w-60 shrink-0 flex-col border-e border-line bg-surface">
        <div className="flex items-center gap-2.5 px-5 py-5">
          {/* The mark is the app's initial in a rounded tile — enough to feel
              like a product, with no asset to ship or to get wrong in RTL. */}
          <span
            aria-hidden="true"
            className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-brand text-sm font-bold text-white"
          >
            T
          </span>
          <span className="flex min-w-0 flex-col leading-tight">
            <span className="truncate text-sm font-semibold text-ink">{t("app.name")}</span>
            <span className="truncate text-xs text-muted">{t("app.tagline")}</span>
          </span>
        </div>

        <nav className="flex flex-1 flex-col gap-0.5 px-3" aria-label={t("nav.home")}>
          {navItems.map(({ to, labelKey, Icon }) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              className={({ isActive }) =>
                [
                  "flex items-center gap-2.5 rounded-md px-2.5 py-2 text-sm transition-colors",
                  isActive
                    ? "bg-brand-soft font-medium text-brand"
                    : "text-ink-soft hover:bg-sunken hover:text-ink",
                ].join(" ")
              }
            >
              {({ isActive }) => (
                <>
                  <Icon
                    className={["size-4 shrink-0", isActive ? "" : "text-faint"].join(" ")}
                    aria-hidden="true"
                  />
                  {t(labelKey)}
                </>
              )}
            </NavLink>
          ))}
        </nav>

        <div className="px-3 pb-4">
          <LanguageSwitcher />
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <ClockWarning />
        <main className="mx-auto w-full max-w-5xl px-8 py-7">
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/calendar" element={<CalendarPage />} />
            <Route path="/candidates" element={<CandidatesPage />} />
            <Route path="/candidates/:id" element={<CandidateDetailPage />} />
            <Route path="/receivables" element={<ReceivablesPage />} />
            <Route path="/cash-flow" element={<CashFlowPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/receipt/:paymentId" element={<ReceiptPage />} />
            <Route path="/print-proof" element={<PrintProofPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </main>
      </div>
    </div>
  );
}
