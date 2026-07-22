import { Pipe, type PipeTransform } from "@angular/core";

const MINUTE = 60;
const HOUR = 3600;
const DAY = 86400;

@Pipe({ name: "age" })
export class AgePipe implements PipeTransform {
  transform(seconds: number | null): string {
    if (seconds === null) {
      return "—";
    }
    if (seconds < MINUTE) {
      return "just now";
    }
    if (seconds < HOUR) {
      return `${Math.floor(seconds / MINUTE)}m`;
    }
    if (seconds < DAY) {
      return `${Math.floor(seconds / HOUR)}h`;
    }
    return `${Math.floor(seconds / DAY)}d`;
  }
}
