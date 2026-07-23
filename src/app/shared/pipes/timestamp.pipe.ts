import { formatDate } from "@angular/common";
import { inject, LOCALE_ID, Pipe, type PipeTransform } from "@angular/core";

// Renders an absolute epoch-millisecond timestamp as a human-readable date.
// Null renders as an em dash so an absent timestamp reads cleanly.
@Pipe({ name: "timestamp" })
export class TimestampPipe implements PipeTransform {
  readonly #locale = inject(LOCALE_ID);

  transform(value: number | null): string {
    if (value === null) {
      return "—";
    }
    return formatDate(value, "medium", this.#locale);
  }
}
