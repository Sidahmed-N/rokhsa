import { useTranslation } from "react-i18next";

import { Money, Num } from "@/components/Num";
import { currentLanguage } from "@/i18n";
import type { DayTotal } from "@/lib/reports";

/**
 * Money in and out, day by day, as paired bars.
 *
 * Hand-drawn rather than pulled from a charting library: it is a few dozen
 * divs, it adds nothing to the installer, it inherits the app's colours, and
 * — the part a library would have cost us — it mirrors for Arabic by itself,
 * because the bars are laid out in document order inside a flex row.
 *
 * Deliberately has no axes, gridlines or tooltips. The question it answers is
 * "was this month busier than usual, and which days did money actually come
 * in", which is a shape, not a reading.
 */
export default function MoneyChart({ days }: { days: DayTotal[] }) {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const peak = Math.max(1, ...days.map((d) => Math.max(d.inCentimes, d.outCentimes)));
  const totalIn = days.reduce((sum, d) => sum + d.inCentimes, 0);
  const totalOut = days.reduce((sum, d) => sum + d.outCentimes, 0);

  // Label only the days that can be read without crowding: the 1st, and then
  // every fifth. A label under all thirty-one is noise at this width.
  const labelled = (index: number) => index === 0 || (index + 1) % 5 === 0;

  const dayNumber = (iso: string) => {
    const [, , d] = iso.split("-");
    return String(Number(d));
  };

  return (
    <div>
      <div className="mb-4 flex flex-wrap items-baseline gap-x-5 gap-y-1">
        <span className="inline-flex items-baseline gap-1.5 text-sm">
          <span className="size-2 rounded-full bg-ok" aria-hidden="true" />
          <span className="text-muted">{t("home.inThisMonth")}</span>
          <Money centimes={totalIn} whole className="font-semibold text-ink" />
        </span>
        <span className="inline-flex items-baseline gap-1.5 text-sm">
          <span className="size-2 rounded-full bg-owed" aria-hidden="true" />
          <span className="text-muted">{t("home.outThisMonth")}</span>
          <Money centimes={totalOut} whole className="font-semibold text-ink" />
        </span>
      </div>

      <div className="flex h-28 items-end gap-[3px]">
        {days.map((day) => (
          <div key={day.day} className="flex h-full flex-1 items-end justify-center gap-[2px]">
            {/* A day with nothing still gets a sliver, so the row reads as a
                timeline rather than as gaps between isolated bars. */}
            <span
              className="w-full rounded-t-[2px] bg-ok/70"
              style={{ height: `${Math.max((day.inCentimes / peak) * 100, 2)}%` }}
              title={`${day.day} · ${day.inCentimes / 100}`}
            />
            <span
              className="w-full rounded-t-[2px] bg-owed/60"
              style={{ height: `${Math.max((day.outCentimes / peak) * 100, 2)}%` }}
              title={`${day.day} · ${day.outCentimes / 100}`}
            />
          </div>
        ))}
      </div>

      <div className="mt-1.5 flex gap-[3px]" aria-hidden="true">
        {days.map((day, index) => (
          <span key={day.day} className="flex-1 text-center text-[0.625rem] text-faint">
            {labelled(index) ? <Num>{dayNumber(day.day)}</Num> : ""}
          </span>
        ))}
      </div>

      <p className="sr-only">
        {t("home.chartSummary", { lang: locale, days: days.length })}
      </p>
    </div>
  );
}
