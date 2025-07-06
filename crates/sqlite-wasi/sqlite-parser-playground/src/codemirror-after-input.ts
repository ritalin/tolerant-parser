import { Annotation, Transaction } from "@codemirror/state";
import { EditorView } from "codemirror";

export type AfterInputMetadataSpec = {
    from: number,
    oldLen: number,
    newLen: number,
    inserted: string,
}

export const AfterInputMetadata = Annotation.define<AfterInputMetadataSpec[]>();

export function trackAfterInput() {
    return [
        trackCompositionInput(),
        trackStandardInput(),
    ];
} 

function trackCompositionInput() {
    let startAt = 0;

    return EditorView.domEventHandlers({
        compositionstart(_ev, view) {
            startAt = view.state.selection.main.head;
        },
        compositionend(ev, view) {
            const text = ev.data?? "";
            const p = startAt;

            setTimeout(() => {
                console.log(`start: ${p}, text: ${text}, length: ${text.length}`);
                view.dispatch({
                    annotations: [
                        Transaction.userEvent.of("input:after.compose"),
                        AfterInputMetadata.of([{ from: p, oldLen: 0, newLen: text.length, inserted: text }])
                    ]
                });
            }, 0);
        }
    });
}

function trackStandardInput() {
    return EditorView.updateListener.of((update) => {
        if (! update.docChanged) return;

        for (const tr of update.transactions) {
            const ev = tr.annotation(Transaction.userEvent);
            if (!ev) continue;
            if (ev.includes(":after")) continue;
            if (ev.startsWith("input.type.compose")) continue;

            let scopes: AfterInputMetadataSpec[] = [];

            tr.changes.iterChanges((fromA: number, toA: number, fromB: number, toB: number, inserted) => {
                scopes.push({ from: fromA, oldLen: toA - fromA, newLen: toB - fromB, inserted: inserted.toString() });
            });

            if (scopes.length > 0) {
                const seq = ev.split(".");
                const next = `${ seq.length == 1 ? `${seq[0]}:after` : `${seq[0]}:after.${seq.slice(1).join(".")}` }`
                update.view.dispatch({
                    annotations: [
                        Transaction.userEvent.of(next),
                        AfterInputMetadata.of(scopes)
                    ]
                })
            }
        }
    })
}

export function onAfterInput(handler: (event: string, document: string, metadata: AfterInputMetadataSpec[]) => void) {
    return EditorView.updateListener.of((update) => {
        for (const tr of update.transactions) {
            const event = tr.annotation(Transaction.userEvent);
            if (!event) continue;

            if (! ["input:after", "delete:after", "move:after", "undo:after"].some(ev => event?.startsWith(ev))) continue;

            const metadata = tr.annotation(AfterInputMetadata);
            if (!metadata) continue;

            handler(event, update.state.doc.toString(), metadata);
        }
    });
}