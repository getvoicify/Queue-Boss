import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
} from "@angular/core";
import type { Capabilities, JobDetail } from "../../../core/models";
import { StateColorDirective } from "../../../shared/directives/state-color.directive";
import { AttemptsPipe } from "../../../shared/pipes/attempts.pipe";
import { JsonPreviewPipe } from "../../../shared/pipes/json-preview.pipe";

interface ExtensionRow {
  key: string;
  value: unknown;
}

// Dumb job-detail panel: timeline, retry readout ("N of M" plus a next-retry
// time — never a "backoff" label), and capability-aware extension rows. An
// extension is only shown when its key is BOTH present on the job and
// advertised by the connection's capabilities. No service or `invoke`.
@Component({
  selector: "app-job-detail",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [StateColorDirective, AttemptsPipe, JsonPreviewPipe],
  template: `
    <section class="job-detail" data-testid="job-detail" aria-label="Job detail">
      <h2>{{ job().id }}</h2>
      <dl class="job-detail__fields">
        <dt>State</dt>
        <dd [appStateColor]="job().state">{{ job().state }}</dd>
        <dt>Attempts</dt>
        <dd data-testid="job-retry">{{ job().retry | attempts }}</dd>
        @if (nextRetryIso(); as iso) {
          <dt>Next retry</dt>
          <dd>
            <time data-testid="job-next-retry" [attr.datetime]="iso">
              {{ job().retry.nextRetryAt }}
            </time>
          </dd>
        }
        <dt>Data</dt>
        <dd data-testid="job-data">{{ job().data | jsonPreview }}</dd>
        <dt>Output</dt>
        <dd data-testid="job-output">{{ job().output | jsonPreview }}</dd>
      </dl>

      <section class="job-detail__timeline" aria-label="Timeline">
        <h3>Timeline</h3>
        <ol>
          @for (event of job().timeline; track $index) {
            <li data-testid="timeline-event" [appStateColor]="event.state">
              <span>{{ event.state }}</span>
              <time [attr.datetime]="event.at">{{ event.at }}</time>
            </li>
          }
        </ol>
      </section>

      @if (extensionRows().length) {
        <section class="job-detail__extensions" aria-label="Extensions">
          <h3>Extensions</h3>
          <dl>
            @for (row of extensionRows(); track row.key) {
              <dt>{{ row.key }}</dt>
              <dd [attr.data-testid]="'job-extension-' + row.key">
                {{ row.value | jsonPreview }}
              </dd>
            }
          </dl>
        </section>
      }
    </section>
  `,
})
export class JobDetailComponent {
  readonly job = input.required<JobDetail>();
  readonly capabilities = input.required<Capabilities>();

  protected readonly nextRetryIso = computed(() => {
    const at = this.job().retry.nextRetryAt;
    return at === null ? null : new Date(at).toISOString();
  });

  protected readonly extensionRows = computed<ExtensionRow[]>(() => {
    const extensions = this.job().extensions;
    const advertised = new Set(this.capabilities().extensions);
    return Object.keys(extensions)
      .filter((key) => advertised.has(key))
      .map((key) => ({ key, value: extensions[key] }));
  });
}
