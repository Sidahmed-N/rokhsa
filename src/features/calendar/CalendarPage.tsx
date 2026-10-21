import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CalendarDays, CalendarRange } from "lucide-react";

import Segmented from "@/components/ui/Segmented";
import { PageHeader } from "@/components/ui/Surface";
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

  return (
    <div>
      <PageHeader title={t("calendar.title")}>
        <Segmented<Tab>
          value={tab}
          onChange={setTab}
          label={t("calendar.title")}
          options={[
            {
              id: "day",
              label: t("calendar.tabDay"),
              icon: <CalendarDays className="size-4" aria-hidden="true" />,
            },
            {
              id: "week",
              label: t("calendar.tabWeek"),
              icon: <CalendarRange className="size-4" aria-hidden="true" />,
            },
          ]}
        />
      </PageHeader>

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
