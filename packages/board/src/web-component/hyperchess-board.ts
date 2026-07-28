/**
 * Web Component for HyperChess board
 * Usage: <hyperchess-board hfen="..." theme="modern"></hyperchess-board>
 */
export class HyperchessBoard extends HTMLElement {
  private hfen: string = '12/abcdefghijkl/mnopqrstuvwx/12/12/12/12/12/12/MNOPQRSTUVWX/ABCDEFGHIJKL/12 w KQkq - 0 1';
  private theme: string = 'modern';
  // Note: HTMLElement already declares a public `shadowRoot` property, which
  // attachShadow() also assigns as a side effect — redeclaring it here as a
  // private field of our own conflicted with the inherited one (incompatible
  // visibility), so we keep our own reference under a different name instead.
  private root: ShadowRoot;

  constructor() {
    super();
    this.root = this.attachShadow({ mode: 'open' });
  }

  connectedCallback() {
    this.hfen = this.getAttribute('hfen') || this.hfen;
    this.theme = this.getAttribute('theme') || this.theme;

    this.render();
  }

  static get observedAttributes() {
    return ['hfen', 'theme'];
  }

  attributeChangedCallback(name: string, oldValue: string, newValue: string) {
    if (oldValue === newValue) return;

    if (name === 'hfen') {
      this.hfen = newValue;
      this.render();
    } else if (name === 'theme') {
      this.theme = newValue;
      this.render();
    }
  }

  private render() {
    this.root.innerHTML = `
      <style>
        :host {
          display: block;
          font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        }

        .board {
          display: grid;
          grid-template-columns: repeat(12, 1fr);
          gap: 0;
          aspect-ratio: 1;
          background: #f0d9b5;
          border: 1px solid #999;
        }
      </style>

      <div class="board">
        <!-- Board rendering coming in Phase 3.0.4 -->
        <div style="grid-column: 1 / -1; padding: 20px; text-align: center;">
          Board rendering coming soon
        </div>
      </div>
    `;
  }
}

// Register the custom element
if (!customElements.get('hyperchess-board')) {
  customElements.define('hyperchess-board', HyperchessBoard);
}
