use crate::frontend::{api::ApiClient, app::{site, GlobalState}, dark_mode::DarkMode};
use leptos::{component, use_context, view, IntoView, RwSignal, SignalWith, *};
use leptos_router::*;

#[component]
pub fn Nav() -> impl IntoView {
    let global_state = use_context::<RwSignal<GlobalState>>().unwrap();
    let logout_action = create_action(move |_| async move {
        ApiClient::get().logout().await.unwrap();
        site().refetch();
    });
    let notification_count = create_resource(
        || (),
        move |_| async move {
            ApiClient::get()
                .notifications_count()
                .await
                .unwrap_or_default()
        },
    );

    let (search_query, set_search_query) = create_signal(String::new());
    let mut dark_mode = expect_context::<DarkMode>();
    view! {
        <nav class="max-sm:navbar p-2.5">
            <div
                id="navbar-start"
                class="max-sm:navbar-start max-sm:flex max-sm:dropdown max-sm:dropdown-bottom max-sm:dropdown-end max-sm:w-full md:h-full"
            >
                <img src="/logo.png" class="m-auto" />
                <h1 class="w-min md:hidden text-3xl font-bold font-serif">
                    {GlobalState::api_client().hostname}
                </h1>
                <div class="flex-grow md:hidden"></div>
                <button tabindex="0" class="btn btn-outline lg:hidden">
                    Menu
                </button>
                <ul
                    tabindex="0"
                    class="menu dropdown-content p-2 max-sm:rounded-box max-sm:z-[1] max-sm:shadow md:h-full"
                >
                    <h1 class="px-4 py-2 text-3xl font-bold font-serif sm:hidden">
                        {GlobalState::api_client().hostname}
                    </h1>
                    <li>
                        <A href="/">"Main Page"</A>
                    </li>
                    <li>
                        <A href="/instance/list">"Instances"</A>
                    </li>
                    <li>
                        <A href="/article/list">"Articles"</A>
                    </li>
                    <Show when=move || global_state.with(|state| state.my_profile.is_some())>
                        <li>
                            <A href="/article/create">"Create Article"</A>
                        </li>
                        <li>
                            <A href="/notifications">
                                "Notifications "
                                <span class="indicator-item indicator-end badge badge-neutral">
                                    {notification_count}
                                </span>
                            </A>
                        </li>
                    </Show>
                    <li>
                        <form
                            class="form-control m-0 p-1"
                            on:submit=move |ev| {
                                ev.prevent_default();
                                let navigate = leptos_router::use_navigate();
                                let query = search_query.get();
                                if !query.is_empty() {
                                    navigate(&format!("/search?query={query}"), Default::default());
                                }
                            }
                        >
                            <input
                                type="text"
                                class="input input-secondary input-bordered input-xs w-full rounded"
                                placeholder="Search"
                                prop:value=search_query
                                on:keyup=move |ev: ev::KeyboardEvent| {
                                    let val = event_target_value(&ev);
                                    set_search_query.update(|v| *v = val);
                                }
                            />

                            <button class="btn btn-xs btn-secondary">Go</button>
                        </form>
                    </li>
                    <div class="divider"></div>
                    <Show
                        when=move || global_state.with(|state| state.my_profile.is_some())
                        fallback=move || {
                            view! {
                                <li>
                                    <A href="/login">"Login"</A>
                                </li>
                                <Transition>
                                <Show when=move || site().get().map(|s| s.config.registration_open).unwrap_or_default()>
                                    <li>
                                        <A href="/register">"Register"</A>
                                    </li>
                                </Show></Transition>
                            }
                        }
                    >

                        {
                            let my_profile = global_state
                                .with(|state| state.my_profile.clone().unwrap());
                            let profile_link = format!("/user/{}", my_profile.person.username);
                            view! {
                                <p class="self-center pb-2">
                                    "Logged in as " <a class="link" href=profile_link>
                                        {my_profile.person.username}
                                    </a>
                                </p>
                                <button
                                    class="btn btn-outline btn-xs w-min self-center"
                                    on:click=move |_| logout_action.dispatch(())
                                >
                                    Logout
                                </button>
                            }
                        }

                    </Show>
                    <div class="flex-grow min-h-2"></div>
                    <div class="m-1 grid gap-2">
                        <label class="flex cursor-pointer gap-2">
                            <span class="label-text">Light</span>
                            <input
                                type="checkbox"
                                class="toggle"
                                prop:checked=dark_mode.is_dark
                                on:click=move |_| { dark_mode.toggle() }
                            />
                            <span class="label-text">Dark</span>
                        </label>
                        <p>"Version "{env!("CARGO_PKG_VERSION")}</p>
                        <p>
                            <a href="https://github.com/Nutomic/ibis" class="link">
                                Source Code
                            </a>
                        </p>
                    </div>
                </ul>
            </div>
        </nav>
    }
}
