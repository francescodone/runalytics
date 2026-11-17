import type { CSSProperties, SVGProps } from "react";
import { AnimatedIcon, type IconName } from "./animated-icons";

type IconProps = Omit<SVGProps<SVGSVGElement>, "stroke"> & { size?: number | string; stroke?: number | string };
function icon(name: IconName) {
  return function Icon({ size = 24, className = "", color, style }: IconProps) {
    const pixels = Number(size);
    return <AnimatedIcon name={name} size={Number.isFinite(pixels) && pixels > 0 ? pixels : 24} className={className.replace(/\bspinner(?:-reverse)?\b/g, "").trim()} style={{ ...style, color: color ?? style?.color } as CSSProperties} />;
  };
}

export const IconAdjustments = icon("adjustments");
export const IconAlertCircle = icon("error");
export const IconArrowDown = icon("arrowDown");
export const IconArrowLeft = icon("back");
export const IconArrowRight = icon("forward");
export const IconArrowUp = icon("arrowUp");
export const IconArrowUpRight = icon("arrowUpRight");
export const IconBarbell = icon("strength");
export const IconBike = icon("bike");
export const IconBrandStrava = icon("strava");
export const IconCalendarEvent = icon("calendar");
export const IconCalendarShare = icon("calendar");
export const IconCalendarWeek = icon("calendar");
export const IconCamera = icon("camera");
export const IconChartLine = icon("chart");
export const IconCheck = icon("check");
export const IconChevronDown = icon("chevronDown");
export const IconChevronLeft = icon("previous");
export const IconChevronRight = icon("next");
export const IconClock = icon("clock");
export const IconCopy = icon("copy");
export const IconDeviceFloppy = icon("save");
export const IconDeviceWatch = icon("watch");
export const IconDownload = icon("download");
export const IconFilter = icon("filter");
export const IconFocus2 = icon("focus");
export const IconFolder = icon("folder");
export const IconHelp = icon("help");
export const IconInfoCircle = icon("info");
export const IconLanguage = icon("language");
export const IconLink = icon("connect");
export const IconLoader2 = icon("pending");
export const IconMinus = icon("minus");
export const IconPalette = icon("palette");
export const IconPlus = icon("plus");
export const IconQrcode = icon("qr");
export const IconRefresh = icon("sync");
export const IconRestore = icon("restore");
export const IconRun = icon("run");
export const IconSearch = icon("search");
export const IconSettings = icon("settings");
export const IconTrash = icon("delete");
export const IconTrophy = icon("milestones");
export const IconUnlink = icon("disconnect");
export const IconUpload = icon("upload");
export const IconX = icon("close");
