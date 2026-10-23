import { NavLink, Navigate, Route, Routes } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  CalendarDays,
  GraduationCap,
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
import ExamsPage from "@/features/exams/ExamsPage";
import HomePage from "@/features/home/HomePage";
import PrintProofPage from "@/features/print/PrintProofPage";
import ReceiptPage from "@/features/print/ReceiptPage";
import CashFlowPage from "@/features/reports/CashFlowPage";
import ReceivablesPage from "@/features/reports/ReceivablesPage";
import SettingsPage from "@/features/settings/SettingsPage";

/**
 * Application shell.
 *
 * A tinted rail beside a white panel. The tint is the whole trick: two
 * surfaces that differ let the content read as raised, where white-on-white
 * read as one flat sheet and made a full screen look empty.
 *
 * Navigation is grouped under quiet section labels rather than run together,
 * because the daily work (the schedule, the candidates, the money) and the
 * occasional work (settings, backup) are not the same kind of thing and
 * should not look it.
 *
 * Every spacing and alignment utility here is a LOGICAL property
 * (ms-/me-/ps-/pe-/start-/end-/text-start), never a physical one, and the
 * rail is simply the first flex child — so the whole layout mirrors for
 * Arabic from the single `dir` attribute on <html>, with the rail moving to
 * the right of the window and the panel to its left.
 */
export default function App() {
  const { t } = useTranslation();

  const groups = [
    {
      labelKey: "nav.groupDaily",
      items: [
        { to: "/", labelKey: "nav.home", Icon: LayoutDashboard },
        { to: "/calendar", labelKey: "nav.calendar", Icon: CalendarDays },
        { to: "/candidates", labelKey: "nav.candidates", Icon: Users },
        { to: "/exams", labelKey: "nav.exams", Icon: GraduationCap },
      ],
    },
    {
      labelKey: "nav.groupMoney",
      items: [
        { to: "/receivables", labelKey: "nav.receivables", Icon: Wallet },
        { to: "/cash-flow", labelKey: "nav.cashFlow", Icon: Receipt },
      ],
    },
    {
      labelKey: "nav.groupSystem",
      items: [{ to: "/settings", labelKey: "nav.settings", Icon: Settings }],
    },
  ] as const;

  return (
    <div className="flex min-h-screen bg-rail">
      <aside className="screen-only sticky top-0 flex h-screen w-60 shrink-0 flex-col">
        <div className="flex items-center gap-2.5 px-5 py-5">
          {/* The mark is the app's initial in a rounded tile — enough to feel
              like a product, with no asset to ship or to get wrong in RTL. */}
          <span
            aria-hidden="true"
            className="flex size-8 shrink-0 items-center justify-center rounded-[0.6rem] bg-brand text-sm font-bold text-white shadow-card"
          >
            T
          </span>
          <span className="flex min-w-0 flex-col leading-tight">
            <span className="truncate text-sm font-semibold text-ink">{t("app.name")}</span>
            <span className="truncate text-xs text-muted">{t("app.tagline")}</span>
          </span>
        </div>

        <nav className="flex flex-1 flex-col gap-5 overflow-y-auto px-3 pt-2" aria-label={t("nav.home")}>
          {groups.map((group) => (
            <div key={group.labelKey}>
              <p className="mb-1.5 px-2.5 text-[0.6875rem] font-medium tracking-wide text-faint uppercase">
                {t(group.labelKey)}
              </p>
              <div className="flex flex-col gap-0.5">
                {group.items.map(({ to, labelKey, Icon }) => (
                  <NavLink
                    key={to}
                    to={to}
                    end={to === "/"}
                    className={({ isActive }) =>
                      [
                        "flex items-center gap-2.5 rounded-full px-3 py-2 text-sm transition-colors",
                        isActive
                          ? "bg-surface font-medium text-ink shadow-card"
                          : "text-rail-ink hover:bg-surface/60",
                      ].join(" ")
                    }
                  >
                    {({ isActive }) => (
                      <>
                        <Icon
                          className={["size-4 shrink-0", isActive ? "text-brand" : "text-muted"].join(
                            " ",
                          )}
                          aria-hidden="true"
                        />
                        {t(labelKey)}
                      </>
                    )}
                  </NavLink>
                ))}
              </div>
            </div>
          ))}
        </nav>

        <div className="px-3 pb-4 pt-3">
          <LanguageSwitcher />
        </div>
      </aside>

      {/* The white panel. Rounded on the side that meets the rail, which is
          the inline start in French and the inline end in Arabic — hence the
          logical corner utilities rather than rounded-l-*. */}
      <div className="flex min-w-0 flex-1 flex-col rounded-ss-2xl rounded-es-2xl border-s border-rail-edge bg-surface shadow-raised">
        <ClockWarning />
        <main className="mx-auto w-full max-w-5xl px-8 py-7">
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/calendar" element={<CalendarPage />} />
            <Route path="/candidates" element={<CandidatesPage />} />
            <Route path="/candidates/:id" element={<CandidateDetailPage />} />
          <Route path="/exams" element={<ExamsPage />} />
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
