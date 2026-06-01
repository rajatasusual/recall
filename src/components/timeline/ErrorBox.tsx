import { VNode } from "preact";

interface ErrorBoxProps {
  error: string | null;
  onDismiss: () => void;
}

export function ErrorBox({ error, onDismiss }: ErrorBoxProps): VNode | null {
  if (!error) return null;

  return (
    <div class="error-box">
      <div style="display:flex;justify-content:space-between;align-items:flex-start;">
        <div style="flex:1;">
          Error:
          <pre
            style="
              margin-top:8px;
              white-space:pre-wrap;
              word-break:break-word;
              font-size:12px;
              overflow:auto;
            "
          >
            {error}
          </pre>
        </div>

        <button
          class="dismiss-btn"
          onClick={onDismiss}
          title="Dismiss error"
        >
          Dismiss
        </button>
      </div>
    </div>
  );
}
