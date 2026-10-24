import { useTranslation } from "react-i18next";

import BackupSection from "@/features/backup/BackupSection";

/**
 * Backups, and nothing else.
 *
 * This was Réglages, and it held the school's identity, the receipt
 * numbering and the price list. All three went: the receipt was removed, the
 * school teaches one licence so there is no list to keep, and the identity
 * fields existed only to head a printed page that no longer exists. What was
 * left was a settings screen whose every control did nothing — worse than no
 * settings screen, because it suggests the software has knobs that matter.
 *
 * So the page is named for the one thing it does. The school's name and
 * agrément come back here, as two or three fields, on the day the first real
 * printout needs them.
 */
export default function BackupPage() {
  const { t } = useTranslation();

  return (
    <div className="max-w-4xl">
      <h1 className="mb-5 text-[1.375rem] font-semibold tracking-tight text-ink">
        {t("nav.backup")}
      </h1>

      <BackupSection />
    </div>
  );
}
