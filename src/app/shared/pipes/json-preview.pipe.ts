import { Pipe, type PipeTransform } from "@angular/core";

const MAX_LENGTH = 200;

// Compact, length-capped JSON rendering for a job's opaque data/output blobs.
// Null and undefined render as an em dash so an absent payload reads cleanly.
@Pipe({ name: "jsonPreview" })
export class JsonPreviewPipe implements PipeTransform {
  transform(value: unknown): string {
    if (value === null || value === undefined) {
      return "—";
    }
    const json = JSON.stringify(value);
    return json.length > MAX_LENGTH ? `${json.slice(0, MAX_LENGTH)}…` : json;
  }
}
