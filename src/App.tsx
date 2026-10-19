import { NavLink, Navigate, Route, Routes } from "react-router-dom";
import { useTranslation } from "react-i18next";

import LanguageSwitcher from "@/components/LanguageSwitcher";
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
 * Every spacing and alignment utility here is a LOGICAL property
 * (ms-/me-/ps-/pe-/start-/end-/text-start), never a physical one. That is what
 * lets the whole layout mirror for Arabic from a single `dir` attribute.
 */
export default function App() {
  const { t } = useTranslation();

  const navItems = [
    { to: "/", labelKey: "nav.home" },
    { to: "/candidates", labelKey: "nav.candidates" },
    { to: "/receivables", labelKey: "nav.receivables" },
    { to: "/cash-flow", labelKey: "nav.cashFlow" },
    { to: "/settings", labelKey: "nav.settings" },
  ] as const;

  return (
    <div className="min-h-screen">
      <header className="screen-only border-b border-line bg-white">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center gap-4 px-4 py-3">
          <div className="flex flex-col">
            <span className="text-lg font-semibold text-brand">{t("app.name")}</span>
            <span className="text-xs text-muted">{t("app.tagline")}</span>
          </div>

          <nav className="flex items-center gap-1" aria-label={t("nav.home")}>
            {navItems.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.to === "/"}
                className={({ isActive }) =>
                  [
                    "rounded-md px-3 py-1.5 text-sm transition-colors",
                    isActive
                      ? "bg-brand-soft font-medium text-brand"
                      : "text-muted hover:bg-brand-soft hover:text-brand",
                  ].join(" ")
                }
              >
                {t(item.labelKey)}
              </NavLink>
            ))}
          </nav>

          <div className="ms-auto">
            <LanguageSwitcher />
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-4 py-6">
        <Routes>
          <Route path="/" element={<HomePage />} />
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
  );
}
