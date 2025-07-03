import * as parser from '../pkg/parser/parser'

let p: parser.parsers.Parser;
let monitor: TextInputMonitor | null = null;
let tree: parser.syntaxes.SyntaxTree | null = null;

export function initialize(element: HTMLDivElement) {
    p = parser.parsers.create();

    const editor = element.querySelector<HTMLTextAreaElement>('#editor')!;
    const result = element.querySelector<HTMLTextAreaElement>('#result')!;
    const fullParseTime = element.querySelector<HTMLTextAreaElement>('#full-parse-time')!;
    const incParseTime = element.querySelector<HTMLTextAreaElement>('#incremental-parse-time')!;

    monitor = new TextInputMonitor(editor, (offset, oldLen, newLen) => {
        const startTime = performance.now();

        if (tree === null) {
            console.log("[FULL]");

            tree = p.parse(editor.value);
            fullParseTime.textContent = `${performance.now() - startTime}msec`
        }
        else {
            console.log(`[INCL] startOffset: ${offset}, oldLen: ${oldLen}, newLen: ${newLen} }`);

            tree = p.incremental(tree, [{
                startOffset: offset,
                oldLen: oldLen,
                newLen: newLen
            }])
            .parse(editor.value);
            incParseTime.textContent = `${performance.now() - startTime}msec`
        }

        if (tree !== null) {
            visualizeTree(result, tree);
        }
    })

    void monitor;
}

function visualizeTree(result: HTMLTextAreaElement, tree: parser.syntaxes.SyntaxTree) {
    const stack = [{ el: { val: tree.root(), tag: 'node' } as parser.syntaxes.SyntaxElement, depth: 0 }];
    const buffer: string[] = [];

    let entry;
    while((entry = stack.pop()) !== undefined) {
        const {el, depth} = entry;
        if (el.tag === 'node') {
            visualizeNode(buffer, el.val.metadataKey(), el.val.metadata(), null, depth);

            stack.push(...el.val.children().map(child => { 
                return { el: child, depth: depth + 1 }
            }).reverse());
        }
        else if (el.tag === 'token-set') {
            visualizeNode(buffer, el.val.metadataKey(), el.val.metadata(), null, depth);

            for (const trivia of el.val.leadingTrivia()) {
                visualizeNode(buffer, trivia.metadataKey(), trivia.metadata(), trivia.value(), depth + 1);
            }

            const token = el.val.token();
            visualizeNode(buffer, token.metadataKey(), token.metadata(), token.value(), depth + 1);

            for (const trivia of el.val.trailingTrivia()) {
                visualizeNode(buffer, trivia.metadataKey(), trivia.metadata(), trivia.value(), depth + 1);
            }
        }

        result.value = buffer.join("\n");
    }
}

function visualizeNode(buffer: string[], key: parser.syntaxes.MetadataKey, metadata: parser.syntaxes.Metadata, value: string | null, depth: number) {
    const byteRange = `(${key.offset}-${key.offset+key.len})`;
    const nodeType = `${metadata.nodeType}${metadata.patch !== 'none' ? `(patch:${metadata.patch})` : ''}`;
    const nodeValue = value ? `"${value}"` : '';

    buffer.push(`${byteRange.padEnd(16)}${nodeType.padEnd(30)} | ${' '.repeat(depth * 2)}${key.kind.name} ${nodeValue}`);    
}

class TextInputMonitor {
  private editor: HTMLTextAreaElement;
  private onChange: (beforeStart: number, beforeLength: number, afterLength: number) => any;
  private isComposing = false;
  private isCtrlAsciiCommand = false;
  private beforeInputStart: number | null = null;
  private beforeInputEnd: number | null = null;
  private lastEditLength: number = 0;

  constructor(
    editor: HTMLTextAreaElement,
    onChange: (beforeStart: number, beforeLength: number, afterLength: number) => any
  ) {
    this.editor = editor;
    this.onChange = onChange;

    this.editor.addEventListener("keydown", this.onKeydown);
    this.editor.addEventListener("compositionstart", this.onCompositionStart);
    this.editor.addEventListener("compositionend", this.onCompositionEnd);
    this.editor.addEventListener("beforeinput", this.onBeforeInput);
    this.editor.addEventListener("input", this.onInput);
  }

  private onKeydown = (ev: KeyboardEvent) => {
    if (this.isComposing) return;
    if (! ev.ctrlKey) return;

    this.isCtrlAsciiCommand = ['d', 'h','k', 'u', 'w'].includes(ev.key.toLowerCase());
  };

  private onCompositionStart = () => {
    this.isComposing = true;
  };

  private onCompositionEnd = () => {
    this.isComposing = false;
  };

  private onBeforeInput = () => {
    if (this.isCtrlAsciiCommand) {
      this.isCtrlAsciiCommand = false;
      setTimeout(() => {
        if (this.lastEditLength === this.editor.value.length) return;

        this.doChange({
          start: this.editor.selectionStart, 
          beforeLength: Math.abs(this.lastEditLength - this.editor.value.length), 
          afterLength: 0,
        });
      }, 0);
      return;
    }

    this.beforeInputStart = this.editor.selectionStart;
    this.beforeInputEnd = this.editor.selectionEnd;
  };

  private onInput = (ev: Event) => {
    if (this.isComposing) return;

    if (this.beforeInputStart !== null && this.beforeInputEnd !== null) {
      let beforeSelection = {
        start: this.beforeInputStart,
        end: this.beforeInputEnd,
      };
      let afterSelection = {
        start: this.editor.selectionStart,
        end: this.editor.selectionEnd,
      };

      if ((ev as InputEvent).inputType  == 'historyUndo') {
        // swap selection
        [beforeSelection.start, afterSelection.start] = [afterSelection.start, beforeSelection.start]
      }
      
      if (afterSelection.start < beforeSelection.start) {
        // Swap beforeLength and afterLength when the selection is reversed 
        this.doChange({
          start: afterSelection.start, 
          beforeLength: beforeSelection.start - afterSelection.start, 
          afterLength: beforeSelection.end - beforeSelection.start,
        });
      }
      else {
        this.doChange({
          start: beforeSelection.start, 
          beforeLength: beforeSelection.end - beforeSelection.start, 
          afterLength: afterSelection.start - beforeSelection.start
        });
      }
    }
  };

  private doChange = ({start, beforeLength, afterLength}: {start: number, beforeLength: number, afterLength: number}) => {
    try {
      this.onChange(start, beforeLength, afterLength);
    }
    finally {
      this.beforeInputStart = null;
      this.beforeInputEnd = null;
      this.lastEditLength = this.editor.value.length;
    }
  };

  dispose() {
    this.editor.removeEventListener("compositionstart", this.onCompositionStart);
    this.editor.removeEventListener("compositionend", this.onCompositionEnd);
    this.editor.removeEventListener("beforeinput", this.onBeforeInput);
    this.editor.removeEventListener("input", this.onInput);
  }
}
