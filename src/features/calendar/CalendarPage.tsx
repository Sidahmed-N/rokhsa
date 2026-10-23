import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CalendarDays, CalendarRange, Grid3x3 } from "lucide-react";

import Segmented from "@/components/ui/Segmented";
import { PageHeader } from "@/components/ui/Surface";
import { todayIso } from "@/lib/training";
import { ScheduleSessionDialog } from "@/features/training/TrainingDialogs";

import DayView from "./DayView";
import PlannerView from "./PlannerView";

type Tab = "day" | "week" | "month";

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
  const [tab, setTab] = useState<Tab>(() => {
    const asked = new URLSearchParams(window.location.search).get("tab");
    return asked === "week" || asked === "month" ? asked : "day";
  });
  const [date, setDate] = useState(todayIso());
  // The slot a click landed on, and therefore whether the dialog is open.
  const [booking, setBooking] = useState<{ day: string; time: string | null } | null>(null);
  // Bumped after a booking so the open view reloads its sessions.
  const [version, setVersion] = useState(0);

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
            {
              id: "month",
              label: t("calendar.tabMonth"),
              icon: <Grid3x3 className="size-4" aria-hidden="true" />,
            },
          ]}
        />
      </PageHeader>

      {tab === "day" ? (
        <DayView
          key={version}
          date={date}
          setDate={setDate}
          onBook={(day) => setBooking({ day, time: null })}
        />
      ) : (
        <PlannerView
          // Remounting on a view change lets FullCalendar own its own
          // initialView rather than being driven through its imperative API.
          key={`${tab}-${version}`}
          view={tab}
          date={date}
          setDate={setDate}
          openDay={(iso) => {
            setDate(iso);
            setTab("day");
          }}
          onPickSlot={(day, time) => setBooking({ day, time })}
        />
      )}

      {booking && (
        <ScheduleSessionDialog
          enrollmentId={null}
          initialDay={booking.day}
          {...(booking.time === null ? {} : { initialTime: booking.time })}
          onClose={() => setBooking(null)}
          onDone={() => {
            setBooking(null);
            setVersion((n) => n + 1);
          }}
        />
      )}
    </div>
  );
}
