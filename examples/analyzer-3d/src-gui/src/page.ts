import { invoke } from "@novonotes/webview-bridge";
import { useEffect, useState } from "react";

export const useEditorPage = () => {
    const [page, setPage] = useState<string | null>(null);
    useEffect(() => {
        invoke("get_editor_page").then((result) => {
            const typed_result = result as { page: string };
            setPage(typed_result.page);
        });
    }, []);

    const _setPage = (page: string) => {
        invoke("set_editor_page", {
            page
        });
        setPage(page);
    }


    return {
        page,
        setPage: _setPage
    }
}