import { App as AntApp, ConfigProvider, theme } from "antd";
import { App } from "./App.tsx";
import { useI18n } from "./i18n.tsx";

export function Providers() {
  const { antdLocale } = useI18n();

  return (
    <ConfigProvider
      locale={antdLocale}
      theme={{
        algorithm: theme.defaultAlgorithm,
        token: {
          borderRadius: 10,
          colorPrimary: "#6c5ce7",
        },
      }}
    >
      <AntApp>
        <App />
      </AntApp>
    </ConfigProvider>
  );
}
