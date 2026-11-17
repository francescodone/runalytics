import refresh from "react-useanimations/lib/refresh";
import loading from "react-useanimations/lib/loading";
import check from "react-useanimations/lib/check";
import alertCircle from "react-useanimations/lib/alertCircle";
import settings from "react-useanimations/lib/settings";
import help from "react-useanimations/lib/help";
import calendar from "react-useanimations/lib/calendar";
import award from "react-useanimations/lib/award";
import arrowLeft from "react-useanimations/lib/arrowLeft";
import arrowRight from "react-useanimations/lib/arrowRight";
import chevronLeft from "react-useanimations/lib/chevronLeft";
import chevronRight from "react-useanimations/lib/chevronRight";
import trash from "react-useanimations/lib/trash";
import filter from "react-useanimations/lib/filter";
import copy from "react-useanimations/lib/copy";
import link from "react-useanimations/lib/link";
import download from "react-useanimations/lib/download";
import upload from "react-useanimations/lib/upload";
import settings2 from "react-useanimations/lib/settings2";
import arrowDown from "react-useanimations/lib/arrowDown";
import arrowUp from "react-useanimations/lib/arrowUp";
import arrowUpRight from "react-useanimations/lib/arrowUpRight";
import camera from "react-useanimations/lib/camera";
import trendingUp from "react-useanimations/lib/trendingUp";
import chevronDown from "react-useanimations/lib/chevronDown";
import clock from "react-useanimations/lib/clock";
import save from "react-useanimations/lib/save";
import watch from "react-useanimations/lib/watch";
import crosshair from "react-useanimations/lib/crosshair";
import folder from "react-useanimations/lib/folder";
import info from "react-useanimations/lib/info";
import minus from "react-useanimations/lib/minus";
import plusCircle from "react-useanimations/lib/plusCircle";
import rotateCcw from "react-useanimations/lib/rotateCcw";
import searchToX from "react-useanimations/lib/searchToX";
import alertTriangle from "react-useanimations/lib/alertTriangle";
import barChart2 from "react-useanimations/lib/barChart2";
import flag from "react-useanimations/lib/flag";
import gift from "react-useanimations/lib/gift";
import map from "react-useanimations/lib/map";
import moon from "react-useanimations/lib/moon";
import repeat from "react-useanimations/lib/repeat";
import layers from "react-useanimations/lib/layers";
import star from "react-useanimations/lib/star";
import sunrise from "react-useanimations/lib/sunrise";
import target from "react-useanimations/lib/target";
import zap from "react-useanimations/lib/zap";
import zoomIn from "react-useanimations/lib/zoomIn";
import zoomOut from "react-useanimations/lib/zoomOut";
import circle from "react-useanimations/lib/circle";
import checkBox from "react-useanimations/lib/checkBox";
import radioButton from "react-useanimations/lib/radioButton";
import toggle from "react-useanimations/lib/toggle";

export const catalogIcons = {
  sync: { animation: refresh, poster: "refresh" },
  pending: { animation: loading, poster: "loading" },
  success: { animation: check, poster: "check" },
  error: { animation: alertCircle, poster: "alertCircle" },
  settings: { animation: settings, poster: "settings" },
  help: { animation: help, poster: "help" },
  calendar: { animation: calendar, poster: "calendar" },
  milestones: { animation: award, poster: "award" },
  back: { animation: arrowLeft, poster: "arrowLeft" },
  forward: { animation: arrowRight, poster: "arrowRight" },
  previous: { animation: chevronLeft, poster: "chevronLeft" },
  next: { animation: chevronRight, poster: "chevronRight" },
  delete: { animation: trash, poster: "trash" },
  filter: { animation: filter, poster: "filter" },
  copy: { animation: copy, poster: "copy" },
  connect: { animation: link, poster: "link" },
  download: { animation: download, poster: "download" },
  upload: { animation: upload, poster: "upload" },
  adjustments: { animation: settings2, poster: "settings2" },
  arrowDown: { animation: arrowDown, poster: "arrowDown" },
  arrowUp: { animation: arrowUp, poster: "arrowUp" },
  arrowUpRight: { animation: arrowUpRight, poster: "arrowUpRight" },
  camera: { animation: camera, poster: "camera" },
  chart: { animation: trendingUp, poster: "trendingUp" },
  check: { animation: check, poster: "check" },
  chevronDown: { animation: chevronDown, poster: "chevronDown" },
  clock: { animation: clock, poster: "clock" },
  save: { animation: save, poster: "save" },
  watch: { animation: watch, poster: "watch" },
  focus: { animation: crosshair, poster: "crosshair" },
  folder: { animation: folder, poster: "folder" },
  info: { animation: info, poster: "info" },
  minus: { animation: minus, poster: "minus" },
  plus: { animation: plusCircle, poster: "plusCircle" },
  restore: { animation: rotateCcw, poster: "rotateCcw" },
  search: { animation: searchToX, poster: "searchToX" },
  warning: { animation: alertTriangle, poster: "alertTriangle" },
  barChart: { animation: barChart2, poster: "barChart2" },
  flag: { animation: flag, poster: "flag" },
  gift: { animation: gift, poster: "gift" },
  map: { animation: map, poster: "map" },
  moon: { animation: moon, poster: "moon" },
  repeat: { animation: repeat, poster: "repeat" },
  layers: { animation: layers, poster: "layers" },
  star: { animation: star, poster: "star" },
  sunrise: { animation: sunrise, poster: "sunrise" },
  target: { animation: target, poster: "target" },
  zap: { animation: zap, poster: "zap" },
  zoomIn: { animation: zoomIn, poster: "zoomIn" },
  zoomOut: { animation: zoomOut, poster: "zoomOut" },
  circle: { animation: circle, poster: "circle" },
  checkbox: { animation: checkBox, poster: "checkBox" },
  radio: { animation: radioButton, poster: "radioButton" },
  toggle: { animation: toggle, poster: "toggle" },
} as const;
