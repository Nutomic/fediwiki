use crate::frontend::article_title;
use crate::frontend::components::article_nav::ArticleNav;
use crate::frontend::pages::article_resource;
use leptos::*;
use leptos_router::*;

#[component]
pub fn ArticleHistory() -> impl IntoView {
    let params = use_params_map();
    let title = move || params.get().get("title").cloned();
    let article = article_resource(title);

    view! {
        <ArticleNav article=article/>
        <Suspense fallback=|| view! {  "Loading..." }> {
            move || article.get().map(|article| {
                view! {
                    <div class="item-view">
                        <h1>{article_title(&article.article)}</h1>
                        {
                            article.edits.into_iter().rev().map(|edit| {
                                let path = format!("/article/{}/diff/{}", article.article.title, edit.hash.0);
                                // TODO: need to return username from backend and show it
                                let label = format!("{} ({})", edit.summary, edit.created.to_rfc2822());
                                view! {<li><a href={path}>{label}</a></li> }
                            }).collect::<Vec<_>>()
                        }
                    </div>
                }
            })
        }
        </Suspense>
    }
}
