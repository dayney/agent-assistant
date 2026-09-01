import { useEffect, useRef } from 'react';
import type { RuleTarget } from '../core/model';

interface RuleConflictSheetProps {
    conflict: RuleTarget | null;
    blockedTargets: RuleTarget[];
    backupConfirmation: boolean;
    busy: boolean;
    onClose: () => void;
    onImportNative: () => void;
    onContinueToBackup: () => void;
    onBackupOverwrite: () => void;
}

export function RuleConflictSheet({ conflict, blockedTargets, backupConfirmation, busy, onClose, onImportNative, onContinueToBackup, onBackupOverwrite }: RuleConflictSheetProps) {
    const reviewDialogRef = useRef<HTMLDialogElement>(null);
    const backupDialogRef = useRef<HTMLDialogElement>(null);
    const ignoreReviewCloseRef = useRef(false);

    useEffect(() => {
        const dialog = reviewDialogRef.current;
        if (!dialog) return;
        if (conflict && !backupConfirmation && !dialog.open) dialog.showModal();
        if ((!conflict || backupConfirmation) && dialog.open) {
            ignoreReviewCloseRef.current = backupConfirmation;
            dialog.close();
        }
    }, [backupConfirmation, conflict]);

    useEffect(() => {
        const dialog = backupDialogRef.current;
        if (!dialog) return;
        if (conflict && backupConfirmation && !dialog.open) dialog.showModal();
        if ((!conflict || !backupConfirmation) && dialog.open) dialog.close();
    }, [backupConfirmation, conflict]);

    function handleReviewClose() {
        if (ignoreReviewCloseRef.current) {
            ignoreReviewCloseRef.current = false;
            return;
        }
        onClose();
    }

    return <>
        <dialog ref={reviewDialogRef} className="conflict-dialog" aria-labelledby="conflict-title" onClose={handleReviewClose}>
            {conflict && <><div className="dialog-heading"><div><p className="eyebrow">Sync Blocked</p><h2 id="conflict-title">{conflict.agent} 存在未同步修改</h2></div></div><p className="dialog-copy">同步已经停止。先审查原生差异，再选择导入母版或继续到备份确认。</p><code className="dialog-path">{conflict.path || conflict.reason || '无目标路径'}</code><pre className="diff-view">{conflict.diff || conflict.reason || '没有可显示的差异。'}</pre><div className="dialog-actions"><button className="button button-secondary" type="button" disabled={busy} onClick={onClose}>取消</button>{conflict.supported && <button className="button button-secondary" type="button" disabled={busy} onClick={onImportNative}>导入原生内容</button>}<button className="button button-danger" type="button" disabled={busy} onClick={onContinueToBackup}>继续到备份</button></div></>}
        </dialog>
        <dialog ref={backupDialogRef} className="conflict-dialog" aria-labelledby="backup-title" onClose={onClose}>
            <div className="dialog-heading"><div><p className="eyebrow">Backup and Overwrite</p><h2 id="backup-title">备份并覆盖原生 Rule</h2></div></div><p className="dialog-copy">此操作会先备份下列原生文件，再以当前母版覆盖。按 Return 不会执行覆盖。</p><ul>{blockedTargets.map((target) => <li key={target.agent}><strong>{target.agent}</strong> <code>{target.path || target.reason || '无目标路径'}</code></li>)}</ul><div className="dialog-actions"><button className="button button-secondary" type="button" disabled={busy} onClick={onClose}>取消</button><button className="button button-danger" type="button" disabled={busy} onClick={onBackupOverwrite}>备份并覆盖</button></div>
        </dialog>
    </>;
}
