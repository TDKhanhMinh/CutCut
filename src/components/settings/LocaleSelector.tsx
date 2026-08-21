import { useI18n, type Locale } from "@/i18n";

export function LocaleSelector() {
  const { locale, setLocale, t } = useI18n();
  return (
    <section className="max-w-xl rounded-lg border border-border bg-card p-5">
      <h2 className="mb-2 text-lg font-semibold">{t("common.language")}</h2>
      <p className="mb-4 text-sm text-muted-foreground">{t("settings.languageDescription")}</p>
      <label className="block text-sm">
        <span className="sr-only">{t("common.language")}</span>
        <select
          value={locale}
          onChange={(event) => setLocale(event.target.value as Locale)}
          className="rounded-md border border-border bg-background px-3 py-2"
        >
          <option value="vi">{t("common.vietnamese")}</option>
          <option value="en">{t("common.english")}</option>
        </select>
      </label>
    </section>
  );
}
