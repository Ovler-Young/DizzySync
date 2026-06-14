import { Alert, Card, Collapse, Typography } from "antd";
import { useI18n } from "../i18n.tsx";

const { Paragraph, Text } = Typography;

export function ConfigGuide() {
  const { t } = useI18n();

  return (
    <Card title={t("guide.title")}>
      <Alert
        showIcon={true}
        style={{ marginBottom: 16 }}
        type="warning"
        message={t("guide.credentials.title")}
        description={t("guide.credentials.description")}
      />
      <Collapse
        items={[
          {
            key: "user",
            label: t("guide.user.label"),
            children: <Paragraph>{t("guide.user.body")}</Paragraph>,
          },
          {
            key: "download",
            label: t("guide.download.label"),
            children: (
              <ul className="guide-list">
                <li>128: 128kbps MP3.</li>
                <li>320: 320kbps MP3.</li>
                <li>FLAC.</li>
                <li>gift.</li>
                <li>{t("guide.download.conflict")}</li>
              </ul>
            ),
          },
          {
            key: "paths",
            label: t("guide.paths.label"),
            children: (
              <Paragraph>
                {t("guide.paths.body")}
                <br />
                <Text code={true}>{"{album}"}</Text> <Text code={true}>{"{label}"}</Text>{" "}
                <Text code={true}>{"{authors}"}</Text> <Text code={true}>{"{year}"}</Text>{" "}
                <Text code={true}>{"{date}"}</Text>
              </Paragraph>
            ),
          },
          {
            key: "behavior",
            label: t("guide.behavior.label"),
            children: (
              <ul className="guide-list">
                <li>{t("guide.behavior.skipExisting")}</li>
                <li>{t("guide.behavior.singleThreaded")}</li>
                <li>{t("guide.behavior.maxConcurrent")}</li>
                <li>{t("guide.behavior.metadata")}</li>
                <li>{t("guide.behavior.debug")}</li>
              </ul>
            ),
          },
          {
            key: "api",
            label: t("guide.api.label"),
            children: <Paragraph>{t("guide.api.body")}</Paragraph>,
          },
        ]}
      />
    </Card>
  );
}
