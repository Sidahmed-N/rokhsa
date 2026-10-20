import { useState } from "react";
import { useTranslation } from "react-i18next";

import { todayIso } from "@/lib/training";

import DayView from "./DayView";
import WeekView from "./WeekView";

type Tab = "day" | "week";

/**
 * The schedule.
 *
 * Two views over one date: the day the school is working through, and the
 * week it is planning. The date is held here rather than in either view, so
 * moving between the tabs keeps the day you were looking at — the owner who
 * spots a gap on Thursday in the week can switch to it and fill it.
 *
 * It opens on the day, every time. The morning routine (open it, mark each
 * session Fait or Absent) is what keeps the hours current, and every click
 * between the owner and that routine is a reason to skip it.
 */
export default function CalendarPage() {
  const { t } = useTranslation();

  // `?tab=week` opens straight on the week, so the view can be linked to and
  // checked on screen without clicking through. Anything else means the day.
  const [tab, setTab] = useState<Tab>(() =>
    new URLSearchParams(window.location.search).get("tab") === "week" ? "week" : "day",
  );
  const [date, setDate] = useState(todayIso());

  const tabs: { id: Tab; labelKey: string }[] = [
    { id: "day", labelKey: "calendar.tabDay" },
    { id: "week", labelKey: "calendar.tabWeek" },
  ];

  return (
    <div>
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <h1 className="text-xl font-semibold">{t("calendar.title")}</h1>

        <div
          className="flex rounded-md border border-line bg-white p-0.5"
          role="tablist"
          aria-label={t("calendar.title")}
        >
          {tabs.map((item) => (
            <button
              key={item.id}
              type="button"
              role="tab"
              aria-selected={tab === item.id}
              onClick={() => setTab(item.id)}
              className={[
                "rounded px-3 py-1 text-sm transition-colors",
                tab === item.id
                  ? "bg-brand font-medium text-white"
                  : "text-muted hover:text-brand",
              ].join(" ")}
            >
              {t(item.labelKey)}
            </button>
          ))}
        </div>
      </div>

      {tab === "day" ? (
        <DayView date={date} setDate={setDate} />
      ) : (
        <WeekView
          date={date}
          setDate={setDate}
          openDay={(iso) => {
            setDate(iso);
            setTab("day");
          }}
        />
      )}
    </div>
  );
}
