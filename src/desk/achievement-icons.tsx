import type { ComponentProps } from "react";
import type { Achievement } from "../lib/flow";
import { icons } from "../lib/icons";
import { AnimatedIcon } from "./animated-icons";

const achievementIcons = new Map<string, ComponentProps<typeof AnimatedIcon>["name"]>([
  [icons.flag, "flag"],
  [icons.flame, "flame"],
  [icons.target, "target"],
  [icons.run, "run"],
  [icons.trophy, "milestones"],
  [icons.calendar, "calendar"],
  [icons.watch, "watch"],
  [icons.stack, "layers"],
  [icons.clock, "clock"],
  [icons.medal, "milestones"],
  [icons.chart, "barChart"],
  [icons.bolt, "zap"],
  [icons.mountain, "mountain"],
  [icons.sunrise, "sunrise"],
  [icons.moon, "moon"],
  [icons.contrast, "contrast"],
  [icons.star, "star"],
  [icons.gift, "gift"],
  [icons.alert, "warning"],
  [icons.calendarYear, "calendar"],
  [icons.repeat, "repeat"],
  [icons.bed, "rest"],
  [icons.map, "map"],
  [icons.dumbbell, "strength"],
]);

export function AchievementIcon({ badge, size = 24 }: { badge: Pick<Achievement, "icon">; size?: number }) {
  return <AnimatedIcon name={achievementIcons.get(badge.icon) ?? "milestones"} size={size} />;
}
