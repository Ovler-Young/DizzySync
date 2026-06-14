import { Card, Collapse, Typography } from "antd";
import type { Key } from "react";
import { useI18n } from "../i18n.tsx";

const { Paragraph, Text } = Typography;

export type ConfigGuideSection = "user" | "download" | "paths" | "behavior" | "schedule" | "api";

interface ConfigGuideProps {
  activeKey?: ConfigGuideSection;
  onActiveKeyChange?: (key: ConfigGuideSection) => void;
}

export function ConfigGuide({ activeKey, onActiveKeyChange }: ConfigGuideProps) {
  const { t } = useI18n();

  const itemClassName = (key: ConfigGuideSection) =>
    activeKey === key ? "guide-section-active" : undefined;

  return (
    <Card title={t("guide.title")}>
      <Collapse
        activeKey={activeKey}
        onChange={(key: Key | Key[]) => {
          const nextKey = Array.isArray(key) ? key[0] : key;
          if (typeof nextKey === "string") {
            onActiveKeyChange?.(nextKey as ConfigGuideSection);
          }
        }}
        items={[
          {
            key: "user",
            label: t("guide.user.label"),
            className: itemClassName("user"),
            children: <Paragraph>{t("guide.user.body")}</Paragraph>,
          },
          {
            key: "download",
            label: t("guide.download.label"),
            className: itemClassName("download"),
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
            className: itemClassName("paths"),
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
            className: itemClassName("behavior"),
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
            key: "schedule",
            label: t("guide.schedule.label"),
            className: itemClassName("schedule"),
            children: <Paragraph>{t("guide.schedule.body")}</Paragraph>,
          },
          {
            key: "api",
            label: t("guide.api.label"),
            className: itemClassName("api"),
            children: <Paragraph>{t("guide.api.body")}</Paragraph>,
          },
        ]}
      />
    </Card>
  );
}
