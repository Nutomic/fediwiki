use crate::{
    common::{newtypes::ConflictId, ApiConflict, ArticleView, EditArticleForm, Notification},
    frontend::{
        api::CLIENT,
        components::{
            article_nav::{ActiveTab, ArticleNav},
            editor::EditorView,
        },
        pages::article_resource,
    },
};
use leptos::{html::Textarea, prelude::*};
use leptos_router::hooks::use_params_map;
use leptos_use::{use_textarea_autosize, UseTextareaAutosizeReturn};

#[derive(Clone, PartialEq)]
enum EditResponse {
    None,
    Success,
    Conflict(ApiConflict),
}

const CONFLICT_MESSAGE: &str = "There was an edit conflict. Resolve it manually and resubmit.";

#[component]
pub fn EditArticle() -> impl IntoView {
    let article = article_resource();
    let (edit_response, set_edit_response) = signal(EditResponse::None);
    let (edit_error, set_edit_error) = signal(None::<String>);

    let conflict_id = move || use_params_map().get_untracked().get("conflict_id").clone();
    if let Some(conflict_id) = conflict_id() {
        Action::new(move |conflict_id: &String| {
            let conflict_id = ConflictId(conflict_id.parse().unwrap());
            async move {
                let conflict = CLIENT
                    .notifications_list()
                    .await
                    .unwrap()
                    .into_iter()
                    .filter_map(|n| match n {
                        Notification::EditConflict(c) => Some(c),
                        _ => None,
                    })
                    .find(|c| c.id == conflict_id)
                    .unwrap();
                set_edit_response.set(EditResponse::Conflict(conflict));
                set_edit_error.set(Some(CONFLICT_MESSAGE.to_string()));
            }
        })
        .dispatch(conflict_id);
    }

    let textarea_ref = NodeRef::<Textarea>::new();
    let UseTextareaAutosizeReturn {
        content,
        set_content,
        trigger_resize: _,
    } = use_textarea_autosize(textarea_ref);
    let (summary, set_summary) = signal(String::new());
    let (wait_for_response, set_wait_for_response) = signal(false);
    let button_is_disabled =
        Signal::derive(move || wait_for_response.get() || summary.get().is_empty());
    let submit_action = Action::new(
        move |(new_text, summary, article, edit_response): &(
            String,
            String,
            ArticleView,
            EditResponse,
        )| {
            let new_text = new_text.clone();
            let summary = summary.clone();
            let article = article.clone();
            let resolve_conflict_id = match edit_response {
                EditResponse::Conflict(conflict) => Some(conflict.id),
                _ => None,
            };
            let previous_version_id = match edit_response {
                EditResponse::Conflict(conflict) => conflict.previous_version_id.clone(),
                _ => article.latest_version,
            };
            async move {
                set_edit_error.update(|e| *e = None);
                let form = EditArticleForm {
                    article_id: article.article.id,
                    new_text,
                    summary,
                    previous_version_id,
                    resolve_conflict_id,
                };
                set_wait_for_response.update(|w| *w = true);
                let res = CLIENT.edit_article_with_conflict(&form).await;
                set_wait_for_response.update(|w| *w = false);
                match res {
                    Ok(Some(conflict)) => {
                        set_edit_response.update(|v| *v = EditResponse::Conflict(conflict));
                        set_edit_error.set(Some(CONFLICT_MESSAGE.to_string()));
                    }
                    Ok(None) => {
                        set_edit_response.update(|v| *v = EditResponse::Success);
                    }
                    Err(err) => {
                        let msg = err.to_string();
                        log::warn!("Unable to edit: {msg}");
                        set_edit_error.update(|e| *e = Some(msg));
                    }
                }
            }
        },
    );

    view! {
        <ArticleNav article=article active_tab=ActiveTab::Edit />
        <Show
            when=move || edit_response.get() == EditResponse::Success
            fallback=move || {
                view! {
                    <Suspense fallback=|| {
                        view! { "Loading..." }
                    }>
                        {move || {
                            article
                                .get()
                                .map(|mut article| {
                                    if let EditResponse::Conflict(conflict) = edit_response.get() {
                                        article.article.text = conflict.three_way_merge;
                                        set_summary.set(conflict.summary);
                                    }
                                    set_content.set(article.article.text.clone());
                                    let article_ = article.clone();
                                    view! {
                                        // set initial text, otherwise submit with no changes results in empty text
                                        <div>
                                            {move || {
                                                edit_error
                                                    .get()
                                                    .map(|err| {
                                                        view! { <p style="color:red;">{err}</p> }
                                                    })
                                            }} <EditorView textarea_ref content set_content />
                                            <div class="flex flex-row mr-2">
                                                <input
                                                    type="text"
                                                    class="input input-primary grow me-4"
                                                    placeholder="Edit summary"
                                                    value=summary.get_untracked()
                                                    on:keyup=move |ev| {
                                                        let val = event_target_value(&ev);
                                                        set_summary.update(|p| *p = val);
                                                    }
                                                />

                                                <button
                                                    class="btn btn-primary"
                                                    prop:disabled=move || button_is_disabled.get()
                                                    on:click=move |_| {
                                                        submit_action
                                                            .dispatch((
                                                                content.get(),
                                                                summary.get(),
                                                                article_.clone(),
                                                                edit_response.get(),
                                                            ));
                                                    }
                                                >

                                                    Submit
                                                </button>
                                            </div>
                                        </div>
                                    }
                                })
                        }}

                    </Suspense>
                }
            }
        >

            Edit successful!
        </Show>
    }
}
