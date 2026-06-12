import { ClipboardFormat } from "../../types";

export const COPY_FORMATS: Array<{
  key: string;
  format: ClipboardFormat;
  label: string;
  title: string;
}> = [
  { key: "P", format: "plain_text", label: "Plain", title: "Copy as plain text" },
  { key: "U", format: "uppercase", label: "Upper", title: "Copy uppercase" },
  { key: "L", format: "lowercase", label: "Lower", title: "Copy lowercase" },
  { key: "F", format: "remove_formatting", label: "Clean", title: "Remove formatting and copy" },
  { key: "Q", format: "convert_quotes", label: "Quotes", title: "Convert quotes and copy" },
  { key: "T", format: "strip_tracking_params", label: "Links", title: "Strip URL tracking params and copy" },
];

export const COPY_FORMAT_KEYS: Record<string, ClipboardFormat> = COPY_FORMATS.reduce(
  (acc, action) => {
    acc[action.key.toLowerCase()] = action.format;
    return acc;
  },
  { Enter: "original" } as Record<string, ClipboardFormat>
);
