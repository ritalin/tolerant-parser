import { EditorState } from '@codemirror/state';
import * as parser from '../pkg/parser/parser'
import { EditorView, placeholder } from '@codemirror/view';
import { basicSetup } from 'codemirror';
import { onAfterInput, trackAfterInput, type AfterInputMetadataSpec } from './codemirror-after-input';

let p: parser.parsers.Parser;
let tree: parser.syntaxes.SyntaxTree | null = null;

export function initialize(element: HTMLDivElement) {
    p = parser.parsers.create();

    const editorOwner = element.querySelector<HTMLDivElement>('#editor')!;
    const result = element.querySelector<HTMLTextAreaElement>('#result')!;
    const fullParseTime = element.querySelector<HTMLElement>('#full-parse-time')!;
    const incParseTime = element.querySelector<HTMLElement>('#incremental-parse-time')!;

    const lightTheme = EditorView.theme({
      "&": {
        backgroundColor: "white",
        color: "black",
      },
      ".cm-content": {
        fontFamily: "monospace",
        fontSize: "1em",
      }
    });
    const editorState = EditorState.create({
      extensions: [
        basicSetup,
        lightTheme,
        placeholder("ここにSQLを入力"),
        trackAfterInput(),
        onAfterInput(handleEditorUpdate(result, fullParseTime, incParseTime))
      ]
    });
    const editor = new EditorView({
      state: editorState,
      parent: editorOwner,
    });
    void editor;
}

function handleEditorUpdate(result: HTMLTextAreaElement, fullParseTime: HTMLElement, incParseTime: HTMLElement) {
  return (_event: string, source: string, metadata: AfterInputMetadataSpec[]) => {
    const startTime = performance.now();

    if (tree === null) {
        console.log("[FULL]");

        tree = p.parse(source);
        fullParseTime.textContent = `${performance.now() - startTime}msec`
    }
    else {
        console.log("[INCL]");
        const scopes: parser.parsers.EditScope[] = metadata.map(md => {
          console.log(`startOffset : ${md.from}, oldLen: ${md.oldLen}, newLen: ${md.newLen}`);
          return {
            startOffset: md.from,
            oldLen: md.oldLen,
            newLen: md.newLen
          }
        })

        tree = p.incremental(tree, scopes).parse(source);
        incParseTime.textContent = `${performance.now() - startTime}msec`
    }

    if (tree !== null) {
        visualizeTree(result, tree);
    }
  }
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
