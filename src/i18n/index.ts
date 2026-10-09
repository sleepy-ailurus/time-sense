import { createI18n } from 'vue-i18n';
import zh from './zh';
import en from './en';

const savedLocale = localStorage.getItem('timesense-locale') || 'zh';

const i18n = createI18n({
  legacy: false,
  locale: savedLocale,
  fallbackLocale: 'zh',
  messages: { zh, en },
});

export default i18n;

export function setLocale(locale: string) {
  (i18n.global.locale as any).value = locale;
  localStorage.setItem('timesense-locale', locale);
}

export function getLocale(): string {
  return (i18n.global.locale as any).value;
}
