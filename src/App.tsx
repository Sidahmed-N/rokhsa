import { useEffect, useState } from "react";
import { NavLink, Navigate, Route, Routes } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  CalendarDays,
  PanelLeftClose,
  PanelLeftOpen,
  GraduationCap,
  HardDrive,
  LayoutDashboard,
  Receipt,
  Users,
  Wallet,
} from "lucide-react";

import ClockWarning from "@/components/ClockWarning";
import LanguageSwitcher from "@/components/LanguageSwitcher";
import { currentLanguage, setLanguage, type Language } from "@/i18n";
import CalendarPage from "@/features/calendar/CalendarPage";
import CandidateDetailPage from "@/features/candidates/CandidateDetailPage";
import CandidatesPage from "@/features/candidates/CandidatesPage";
import ExamsPage from "@/features/exams/ExamsPage";
import HomePage from "@/features/home/HomePage";
import CashFlowPage from "@/features/reports/CashFlowPage";
import ReceivablesPage from "@/features/reports/ReceivablesPage";
import BackupPage from "@/features/backup/BackupPage";

/**
 * Whether the rail is collapsed, remembered per device.
 *
 * localStorage, not the database: how wide somebody likes their sidebar is a
 * display preference on one machine, not business data, and it has no place
 * in a school’s backup. Same reasoning the surface theme used before it was
 * removed.
 *
 * Reading is wrapped because a private window or blocked site data throws
 * rather than returning null, and a sidebar must not take the app down.
 */
const RAIL_KEY = "tariqi.rail.collapsed";

function storedCollapsed(): boolean {
  try {
    return localStorage.getItem(RAIL_KEY) === "1";
  } catch {
    return false;
  }
}

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
/** One button that swaps the language, for the collapsed rail. */
function CompactLanguageToggle() {
  const { t } = useTranslation();
  const other: Language = currentLanguage() === "ar" ? "fr" : "ar";
  const label = other === "ar" ? t("common.arabic") : t("common.french");

  return (
    <button
      type="button"
      onClick={() => void setLanguage(other)}
      aria-label={label}
      title={label}
      className="flex h-8 w-full items-center justify-center rounded-lg border border-line bg-sunken text-xs font-medium text-muted transition-colors hover:text-ink"
    >
      {/* Two letters, in the script of the language being offered: a
          secretary who reads only one of them still recognises hers. */}
      {other === "ar" ? "ع" : "FR"}
    </button>
  );
}

export default function App() {
  const { t } = useTranslation();
  const [collapsed, setCollapsed] = useState(storedCollapsed);

  useEffect(() => {
    try {
      localStorage.setItem(RAIL_KEY, collapsed ? "1" : "0");
    } catch {
      // A preference that cannot be saved is not worth an error.
    }
  }, [collapsed]);

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
      items: [{ to: "/backup", labelKey: "nav.backup", Icon: HardDrive }],
    },
  ] as const;

  return (
    <div className="flex min-h-screen bg-rail">
      {/* Width is the one layout property animated anywhere in this app.
          CLAUDE.md bans animating layout in a LIST, where every row costs a
          reflow on a 4 GB machine; this is one element for 200ms, and there
          is no way to slide a rail without it. overflow-hidden lets the
          labels be clipped by the narrowing rail instead of disappearing on
          the last frame, which is what makes it read as sliding shut. */}
      <aside
        className={[
          "screen-only sticky top-0 flex h-screen shrink-0 flex-col overflow-hidden",
          "transition-[width] duration-200 ease-out motion-reduce:transition-none",
          collapsed ? "w-16" : "w-60",
        ].join(" ")}
      >
        <div
          className={[
            "flex items-center gap-2 py-5",
            collapsed ? "justify-center px-3" : "px-4",
          ].join(" ")}
        >
          {/* The mark is the app’s initial in a rounded tile — enough to feel
              like a product, with no asset to ship or to get wrong in RTL.
              It stands down when collapsed: 64px holds one control, and a
              control you can press is worth more than a logo. */}
          {!collapsed && (
            <span
              aria-hidden="true"
              className="flex size-8 shrink-0 items-center justify-center rounded-[0.6rem] bg-brand text-sm font-bold text-white shadow-card"
            >
              T
            </span>
          )}
          {!collapsed && (
            <span className="flex min-w-0 flex-col leading-tight">
              <span className="truncate text-sm font-semibold text-ink">{t("app.name")}</span>
              <span className="truncate text-xs text-muted">{t("app.tagline")}</span>
            </span>
          )}

          <button
            type="button"
            onClick={() => setCollapsed((v) => !v)}
            aria-label={collapsed ? t("nav.expandRail") : t("nav.collapseRail")}
            title={collapsed ? t("nav.expandRail") : t("nav.collapseRail")}
            className={[
              "flex size-8 shrink-0 items-center justify-center rounded-lg text-muted",
              "transition-colors hover:bg-surface/70 hover:text-ink",
              collapsed ? "" : "ms-auto",
            ].join(" ")}
          >
            {/* rtl:rotate-180 so the arrow points at the window edge the rail
                actually sits against, which is the right one in Arabic. */}
            {collapsed ? (
              <PanelLeftOpen className="size-4 rtl:rotate-180" aria-hidden="true" />
            ) : (
              <PanelLeftClose className="size-4 rtl:rotate-180" aria-hidden="true" />
            )}
          </button>
        </div>

        {/* overflow-x-hidden is load-bearing: setting only overflow-y makes
            overflow-x compute to auto, and the labels still mounted for the
            collapse animation then pushed a horizontal scrollbar across the
            foot of the rail. */}
        <nav
          className="flex flex-1 flex-col gap-5 overflow-y-auto overflow-x-hidden px-3 pt-2"
          aria-label={t("nav.home")}
        >
          {groups.map((group) => (
            <div key={group.labelKey}>
              <p
                className={[
                  "mb-1.5 px-2.5 text-[0.6875rem] font-medium tracking-wide text-faint uppercase",
                  "transition-opacity duration-150 motion-reduce:transition-none",
                  collapsed ? "opacity-0" : "opacity-100",
                ].join(" ")}
                aria-hidden={collapsed}
              >
                {t(group.labelKey)}
              </p>
              <div className="flex flex-col gap-0.5">
                {group.items.map(({ to, labelKey, Icon }) => (
                  <NavLink
                    key={to}
                    to={to}
                    end={to === "/"}
                    title={collapsed ? t(labelKey) : undefined}
                    className={({ isActive }) =>
                      [
                        "flex items-center gap-2.5 rounded-full py-2 text-sm transition-colors",
                        collapsed ? "justify-center px-2" : "px-3",
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
                        <span className="truncate">{t(labelKey)}</span>
                      </>
                    )}
                  </NavLink>
                ))}
              </div>
            </div>
          ))}
        </nav>

        <div className={collapsed ? "px-2 pb-4 pt-3" : "px-3 pb-4 pt-3"}>
          {/* The pair of labels does not fit in 64px, but losing the language
              switch to a collapsed rail would be a worse trade: two people
              sharing one office PC read different scripts. Collapsed, it
              becomes one button showing the language you would switch TO,
              written in its own script. */}
          {collapsed ? <CompactLanguageToggle /> : <LanguageSwitcher />}
        </div>
      </aside>

      {/* The white panel. Rounded on the side that meets the rail, which is
          the inline start in French and the inline end in Arabic — hence the
          logical corner utilities rather than rounded-l-*. */}
      <div className="flex min-w-0 flex-1 flex-col rounded-ss-2xl rounded-es-2xl border-s border-rail-edge bg-surface shadow-raised">
        <ClockWarning />
        {/* No reading-width cap. This is a dense business app — a week grid
            of seven columns, a table of thirty candidates, a list of debts —
            and almost no prose, so the measure argument that justifies a
            narrow column does not apply. Capped at 5xl the content sat in a
            1024px strip with the rest of a maximised window empty beside it.

            A page that genuinely wants a narrow column states it itself: a
            form is still max-w-3xl, because a name field 1900px wide is not
            an improvement. The shell fills the window; the page decides. */}
        <main className="w-full px-8 py-7">
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/calendar" element={<CalendarPage />} />
            <Route path="/candidates" element={<CandidatesPage />} />
            <Route path="/candidates/:id" element={<CandidateDetailPage />} />
          <Route path="/exams" element={<ExamsPage />} />
            <Route path="/receivables" element={<ReceivablesPage />} />
            <Route path="/cash-flow" element={<CashFlowPage />} />
            <Route path="/backup" element={<BackupPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </main>
      </div>
    </div>
  );
}
