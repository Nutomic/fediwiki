use crate::common::ListArticlesData;
use crate::frontend::app::GlobalState;
use crate::frontend::{article_link, article_title};
use leptos::*;
use web_sys::wasm_bindgen::JsCast;

#[component]
pub fn ListArticles() -> impl IntoView {
    let (only_local, set_only_local) = create_signal(true);
    let articles = create_resource(
        move || only_local.get(),
        |only_local| async move {
            GlobalState::api_client()
                .list_articles(ListArticlesData {
                    only_local: Some(only_local),
                })
                .await
                .unwrap()
        },
    );

    view! {
        <h1>Most recently edited Articles</h1>
        <Suspense fallback=|| view! {  "Loading..." }>
            <fieldset on:input=move |ev| {
                let val = ev
                    .target()
                    .unwrap()
                    .unchecked_into::<web_sys::HtmlInputElement>()
                    .id();
                let is_local_only = val == "only-local";
                set_only_local.update(|p| *p = is_local_only);
            }>
                <input type="radio" name="listing-type" id="only-local" checked />
                <label for="only-local">Only Local</label>
                <input type="radio" name="listing-type" id="all"/>
                <label for="all">All</label>
            </fieldset>
            <ul> {
                move || articles.get().map(|a|
                    a.into_iter().map(|a| view! {
                    <li><a href=article_link(&a)>{article_title(&a)}</a></li>
                }).collect::<Vec<_>>())
            } </ul>
        </Suspense>
    }
}
