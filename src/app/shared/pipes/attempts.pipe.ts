import { Pipe, type PipeTransform } from "@angular/core";
import type { RetryReadout } from "../../core/models";

// Renders a retry readout as "N of M", or just "N" when the backend does not
// advertise a max-attempts ceiling.
@Pipe({ name: "attempts" })
export class AttemptsPipe implements PipeTransform {
  transform(retry: RetryReadout): string {
    return retry.maxAttempts === null
      ? `${retry.attempts}`
      : `${retry.attempts} of ${retry.maxAttempts}`;
  }
}
