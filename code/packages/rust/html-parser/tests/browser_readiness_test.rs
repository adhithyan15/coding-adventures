// These integration tests build browser-readiness fixtures via helper fns that
// mirror large descriptor structs; the many parameters are inherent to the test
// data, so allow the too_many_arguments lint for this test file.
#![allow(clippy::too_many_arguments)]

use coding_adventures_html_parser::{
    parse_browser_document, BrowserActivationDescriptor, BrowserAnchor, BrowserAnchorDescriptor,
    BrowserAnimationInteractionDescriptor, BrowserAriaCollection, BrowserAriaCollectionDescriptor,
    BrowserAriaCollectionItem, BrowserAriaDescriptionDescriptor, BrowserAriaLiveRegion,
    BrowserAriaNameDescriptor, BrowserAriaRange, BrowserAriaRelationDescriptor,
    BrowserCanvasDescriptor, BrowserClipboardInteractionDescriptor, BrowserCommandElement,
    BrowserComponentHydrationDescriptor, BrowserComponentHydrationTarget,
    BrowserCompositionInteractionDescriptor, BrowserContextMenuInteractionDescriptor,
    BrowserCustomElementDescriptor, BrowserDataAttribute, BrowserDataAttributeDescriptor,
    BrowserDatalistOption, BrowserDisclosure, BrowserDisclosureStateDescriptor, BrowserDocument,
    BrowserDocumentMetadata, BrowserDocumentPolicyDescriptor, BrowserDragDropDescriptor,
    BrowserEmbeddedContext, BrowserEmbeddedPolicyDescriptor, BrowserEventHandlerDescriptor,
    BrowserFetchPolicyDescriptor, BrowserFocusNavigationDescriptor, BrowserForm,
    BrowserFormAssociationDescriptor, BrowserFormAutofillDescriptor, BrowserFormButton,
    BrowserFormChoiceControl, BrowserFormControl, BrowserFormControlDescriptor,
    BrowserFormDatalist, BrowserFormFieldset, BrowserFormFileControl, BrowserFormHiddenControl,
    BrowserFormImageControl, BrowserFormLabel, BrowserFormMeasurement, BrowserFormObject,
    BrowserFormObjectParam, BrowserFormOutput, BrowserFormPolicyDescriptor,
    BrowserFormPolicySubmitterDescriptor, BrowserFormResetDescriptor, BrowserFormSelect,
    BrowserFormSubmissionDescriptor, BrowserFormSubmitter, BrowserFormSuccessfulControl,
    BrowserFormTextEntry, BrowserFormValidationControl, BrowserFormValidationDescriptor,
    BrowserFullscreenInteractionDescriptor, BrowserGlobalStateDescriptor, BrowserHeading,
    BrowserHeadingDescriptor, BrowserHttpEquivHint, BrowserImage, BrowserImageCandidateDescriptor,
    BrowserImageMap, BrowserImageMapArea, BrowserImageMapDescriptor, BrowserImageSource,
    BrowserInputPlanningDescriptor, BrowserInteractiveElement,
    BrowserKeyboardInteractionDescriptor, BrowserLifecycleEventDescriptor, BrowserLink,
    BrowserLinkResourceDescriptor, BrowserLoadingHintDescriptor, BrowserMedia,
    BrowserMediaPlaybackDescriptor, BrowserMediaResourceDescriptor, BrowserMediaSource,
    BrowserMediaTrack, BrowserMeta, BrowserMetadataDirective, BrowserNavigationGroup,
    BrowserNavigationGroupDescriptor, BrowserNavigationTargetDescriptor,
    BrowserPointerInteractionDescriptor, BrowserPopover, BrowserPopoverDescriptor,
    BrowserPopoverInvoker, BrowserRefresh, BrowserResource, BrowserResourceEndpointDescriptor,
    BrowserResourceHint, BrowserScript, BrowserScriptExecutionDescriptor,
    BrowserScriptModuleGraphDescriptor, BrowserScriptStorageAccessDescriptor,
    BrowserScriptWorkerMessagingDescriptor, BrowserScrollInteractionDescriptor,
    BrowserSectionLandmark, BrowserSectionLandmarkDescriptor, BrowserSelectOption,
    BrowserSelectionInteractionDescriptor, BrowserSlotDescriptor, BrowserStructuredDataDescriptor,
    BrowserStructuredItem, BrowserStructuredProperty, BrowserStylesheet,
    BrowserStylesheetPlanningDescriptor, BrowserTable, BrowserTableCell,
    BrowserTableStructureDescriptor, BrowserTemplate, BrowserTemplateDescriptor,
    BrowserTextFlowDescriptor, BrowserTextSemantic, BrowserTextSemanticDescriptor,
    BrowserThemeColor,
};
use serde::Deserialize;

const BROWSER_READINESS_FIXTURE: &str = include_str!("fixtures/html-browser-readiness.json");
const BROWSER_READINESS_TEST_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/browser_readiness_test.rs"
));
const HTML_PARSER_SOURCE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));

struct BrowserReadinessCompletionExpectation {
    field: &'static str,
    type_name: &'static str,
    fixture_case: Option<&'static str>,
    focused_tests: &'static [&'static str],
}

const BROWSER_READINESS_COMPLETION_SURFACE: &[BrowserReadinessCompletionExpectation] = &[
    completion_fixture(
        "event_handler_descriptors",
        "BrowserEventHandlerDescriptor",
        "event-handler-page",
    ),
    completion_tests(
        "lifecycle_event_descriptors",
        "BrowserLifecycleEventDescriptor",
        &["browser_lifecycle_event_descriptors_track_load_and_error_recovery_hooks"],
    ),
    completion_tests(
        "animation_interaction_descriptors",
        "BrowserAnimationInteractionDescriptor",
        &["browser_animation_interaction_descriptors_track_css_timeline_hooks"],
    ),
    completion_tests(
        "fullscreen_interaction_descriptors",
        "BrowserFullscreenInteractionDescriptor",
        &["browser_fullscreen_interaction_descriptors_track_embedded_policy_hints"],
    ),
    completion_tests(
        "context_menu_interaction_descriptors",
        "BrowserContextMenuInteractionDescriptor",
        &["browser_context_menu_interaction_descriptors_track_menu_invokers_and_handlers"],
    ),
    completion_fixture(
        "script_execution_descriptors",
        "BrowserScriptExecutionDescriptor",
        "script-style-loading-page",
    ),
    completion_fixture(
        "script_storage_access_descriptors",
        "BrowserScriptStorageAccessDescriptor",
        "script-storage-access-page",
    ),
    completion_fixture(
        "script_worker_messaging_descriptors",
        "BrowserScriptWorkerMessagingDescriptor",
        "script-worker-messaging-page",
    ),
    completion_fixture(
        "script_module_graph_descriptors",
        "BrowserScriptModuleGraphDescriptor",
        "script-module-graph-page",
    ),
    completion_fixture(
        "stylesheet_planning_descriptors",
        "BrowserStylesheetPlanningDescriptor",
        "script-style-loading-page",
    ),
    completion_fixture(
        "document_policy_descriptors",
        "BrowserDocumentPolicyDescriptor",
        "document-metadata-policy-page",
    ),
    completion_fixture(
        "loading_hint_descriptors",
        "BrowserLoadingHintDescriptor",
        "link-resource-metadata-page",
    ),
    completion_fixture(
        "fetch_policy_descriptors",
        "BrowserFetchPolicyDescriptor",
        "link-resource-metadata-page",
    ),
    completion_fixture(
        "resource_endpoint_descriptors",
        "BrowserResourceEndpointDescriptor",
        "document-metadata-policy-page",
    ),
    completion_fixture(
        "link_resource_descriptors",
        "BrowserLinkResourceDescriptor",
        "link-resource-metadata-page",
    ),
    completion_fixture(
        "form_policy_descriptors",
        "BrowserFormPolicyDescriptor",
        "catalog-form-table-page",
    ),
    completion_tests(
        "form_control_descriptors",
        "BrowserFormControlDescriptor",
        &[
            "browser_form_control_descriptors_track_flat_control_inventory",
            "browser_form_control_descriptors_track_missing_names_and_blockers",
        ],
    ),
    completion_tests(
        "form_association_descriptors",
        "BrowserFormAssociationDescriptor",
        &["browser_form_association_descriptors_track_flat_owner_and_label_links"],
    ),
    completion_tests(
        "form_autofill_descriptors",
        "BrowserFormAutofillDescriptor",
        &["browser_form_autofill_descriptors_track_flat_autocomplete_hints_and_blockers"],
    ),
    completion_tests(
        "form_submission_descriptors",
        "BrowserFormSubmissionDescriptor",
        &["browser_form_submission_descriptors_track_flat_successful_controls_and_submitters"],
    ),
    completion_tests(
        "form_reset_descriptors",
        "BrowserFormResetDescriptor",
        &["browser_form_reset_descriptors_track_flat_resetters_and_controls"],
    ),
    completion_tests(
        "form_validation_descriptors",
        "BrowserFormValidationDescriptor",
        &["browser_form_validation_descriptors_track_flat_candidates_and_bypass_hints"],
    ),
    completion_fixture(
        "anchor_descriptors",
        "BrowserAnchorDescriptor",
        "legacy-directory-page",
    ),
    completion_fixture(
        "heading_descriptors",
        "BrowserHeadingDescriptor",
        "legacy-directory-page",
    ),
    completion_fixture(
        "text_semantic_descriptors",
        "BrowserTextSemanticDescriptor",
        "inline-semantic-metadata-page",
    ),
    completion_tests(
        "text_flow_descriptors",
        "BrowserTextFlowDescriptor",
        &[
            "browser_text_flow_descriptors_track_lists_quotes_and_preformatted_blocks",
            "browser_text_flow_descriptors_track_empty_and_unresolved_blockers",
        ],
    ),
    completion_fixture(
        "navigation_target_descriptors",
        "BrowserNavigationTargetDescriptor",
        "legacy-directory-page",
    ),
    completion_fixture(
        "navigation_group_descriptors",
        "BrowserNavigationGroupDescriptor",
        "navigation-menu-descriptor-page",
    ),
    completion_fixture(
        "section_landmark_descriptors",
        "BrowserSectionLandmarkDescriptor",
        "document-outline-landmark-page",
    ),
    completion_fixture(
        "activation_descriptors",
        "BrowserActivationDescriptor",
        "interactive-element-state-page",
    ),
    completion_tests(
        "popover_descriptors",
        "BrowserPopoverDescriptor",
        &[
            "browser_popover_descriptors_track_hosts_invokers_and_actions",
            "browser_popover_descriptors_track_missing_and_invalid_blockers",
        ],
    ),
    completion_fixture(
        "aria_collections",
        "BrowserAriaCollection",
        "aria-collection-descriptor-page",
    ),
    completion_fixture(
        "aria_collection_descriptors",
        "BrowserAriaCollectionDescriptor",
        "aria-collection-descriptor-page",
    ),
    completion_fixture(
        "aria_ranges",
        "BrowserAriaRange",
        "aria-range-descriptor-page",
    ),
    completion_fixture(
        "aria_live_regions",
        "BrowserAriaLiveRegion",
        "aria-live-region-descriptor-page",
    ),
    completion_fixture(
        "aria_name_descriptors",
        "BrowserAriaNameDescriptor",
        "aria-name-descriptor-page",
    ),
    completion_fixture(
        "aria_description_descriptors",
        "BrowserAriaDescriptionDescriptor",
        "aria-description-descriptor-page",
    ),
    completion_fixture(
        "aria_relation_descriptors",
        "BrowserAriaRelationDescriptor",
        "aria-relation-descriptor-page",
    ),
    completion_fixture(
        "image_candidate_descriptors",
        "BrowserImageCandidateDescriptor",
        "responsive-image-metadata-page",
    ),
    completion_fixture(
        "image_map_descriptors",
        "BrowserImageMapDescriptor",
        "responsive-image-metadata-page",
    ),
    completion_fixture(
        "media_playback_descriptors",
        "BrowserMediaPlaybackDescriptor",
        "media-playback-poster-page",
    ),
    completion_tests(
        "media_resource_descriptors",
        "BrowserMediaResourceDescriptor",
        &[
            "browser_media_resource_descriptors_track_source_and_text_track_candidates",
            "browser_media_resource_descriptors_track_missing_sources_and_labels",
        ],
    ),
    completion_fixture(
        "embedded_policy_descriptors",
        "BrowserEmbeddedPolicyDescriptor",
        "embedded-resource-page",
    ),
    completion_fixture(
        "focus_navigation_descriptors",
        "BrowserFocusNavigationDescriptor",
        "interactive-element-state-page",
    ),
    completion_fixture(
        "keyboard_interaction_descriptors",
        "BrowserKeyboardInteractionDescriptor",
        "interactive-element-state-page",
    ),
    completion_fixture(
        "input_planning_descriptors",
        "BrowserInputPlanningDescriptor",
        "form-accessibility-document-page",
    ),
    completion_fixture(
        "drag_drop_descriptors",
        "BrowserDragDropDescriptor",
        "interactive-element-state-page",
    ),
    completion_fixture(
        "clipboard_interaction_descriptors",
        "BrowserClipboardInteractionDescriptor",
        "interactive-element-state-page",
    ),
    completion_fixture(
        "selection_interaction_descriptors",
        "BrowserSelectionInteractionDescriptor",
        "interactive-element-state-page",
    ),
    completion_tests(
        "composition_interaction_descriptors",
        "BrowserCompositionInteractionDescriptor",
        &["browser_composition_interaction_descriptors_track_ime_and_input_hooks"],
    ),
    completion_fixture(
        "pointer_interaction_descriptors",
        "BrowserPointerInteractionDescriptor",
        "interactive-element-state-page",
    ),
    completion_tests(
        "scroll_interaction_descriptors",
        "BrowserScrollInteractionDescriptor",
        &["browser_scroll_interaction_descriptors_track_scrollbars_handlers_and_blockers"],
    ),
    completion_fixture(
        "disclosure_state_descriptors",
        "BrowserDisclosureStateDescriptor",
        "interactive-element-state-page",
    ),
    completion_fixture(
        "template_descriptors",
        "BrowserTemplateDescriptor",
        "component-template-page",
    ),
    completion_fixture(
        "slot_descriptors",
        "BrowserSlotDescriptor",
        "component-template-page",
    ),
    completion_fixture(
        "custom_element_descriptors",
        "BrowserCustomElementDescriptor",
        "component-template-page",
    ),
    completion_fixture(
        "canvas_descriptors",
        "BrowserCanvasDescriptor",
        "component-template-page",
    ),
    completion_fixture(
        "component_hydration_descriptors",
        "BrowserComponentHydrationDescriptor",
        "component-template-page",
    ),
    completion_fixture(
        "data_attribute_descriptors",
        "BrowserDataAttributeDescriptor",
        "component-template-page",
    ),
    completion_fixture(
        "global_state_descriptors",
        "BrowserGlobalStateDescriptor",
        "document-shell-global-state-page",
    ),
    completion_fixture(
        "structured_data_descriptors",
        "BrowserStructuredDataDescriptor",
        "structured-data-microdata-page",
    ),
    completion_fixture(
        "table_structure_descriptors",
        "BrowserTableStructureDescriptor",
        "table-cell-descriptor-page",
    ),
];

const fn completion_fixture(
    field: &'static str,
    type_name: &'static str,
    fixture_case: &'static str,
) -> BrowserReadinessCompletionExpectation {
    BrowserReadinessCompletionExpectation {
        field,
        type_name,
        fixture_case: Some(fixture_case),
        focused_tests: &[],
    }
}

const fn completion_tests(
    field: &'static str,
    type_name: &'static str,
    focused_tests: &'static [&'static str],
) -> BrowserReadinessCompletionExpectation {
    BrowserReadinessCompletionExpectation {
        field,
        type_name,
        fixture_case: None,
        focused_tests,
    }
}

#[derive(Debug, Deserialize)]
struct BrowserReadinessSuite {
    format: String,
    suite: String,
    cases: Vec<BrowserReadinessCase>,
}

#[derive(Debug, Deserialize)]
struct BrowserReadinessCase {
    id: String,
    input: String,
    expected: ExpectedBrowserDocument,
}

#[derive(Debug, Deserialize)]
struct ExpectedBrowserDocument {
    title: Option<String>,
    base_href: Option<String>,
    base_target: Option<String>,
    #[serde(default)]
    metadata: ExpectedDocumentMetadata,
    #[serde(default)]
    document_lang: Option<String>,
    #[serde(default)]
    document_dir: Option<String>,
    #[serde(default)]
    body_id: Option<String>,
    #[serde(default)]
    body_classes: Vec<String>,
    #[serde(default)]
    body_lang: Option<String>,
    #[serde(default)]
    body_dir: Option<String>,
    #[serde(default)]
    document_event_handlers: Vec<String>,
    #[serde(default)]
    body_event_handlers: Vec<String>,
    #[serde(default)]
    event_handler_descriptors: Vec<ExpectedEventHandlerDescriptor>,
    #[serde(default)]
    lifecycle_event_descriptors: Option<Vec<ExpectedLifecycleEventDescriptor>>,
    #[serde(default)]
    animation_interaction_descriptors: Option<Vec<ExpectedAnimationInteractionDescriptor>>,
    #[serde(default)]
    fullscreen_interaction_descriptors: Option<Vec<ExpectedFullscreenInteractionDescriptor>>,
    #[serde(default)]
    context_menu_interaction_descriptors: Option<Vec<ExpectedContextMenuInteractionDescriptor>>,
    body_text: String,
    metas: Vec<ExpectedMeta>,
    resources: Vec<ExpectedResource>,
    #[serde(default)]
    scripts: Vec<ExpectedScript>,
    #[serde(default)]
    script_execution_descriptors: Option<Vec<ExpectedScriptExecutionDescriptor>>,
    #[serde(default)]
    script_storage_access_descriptors: Option<Vec<ExpectedScriptStorageAccessDescriptor>>,
    #[serde(default)]
    script_worker_messaging_descriptors: Option<Vec<ExpectedScriptWorkerMessagingDescriptor>>,
    #[serde(default)]
    script_module_graph_descriptors: Option<Vec<ExpectedScriptModuleGraphDescriptor>>,
    #[serde(default)]
    stylesheets: Vec<ExpectedStylesheet>,
    #[serde(default)]
    stylesheet_planning_descriptors: Option<Vec<ExpectedStylesheetPlanningDescriptor>>,
    #[serde(default)]
    document_policy_descriptors: Vec<ExpectedDocumentPolicyDescriptor>,
    #[serde(default)]
    loading_hint_descriptors: Vec<ExpectedLoadingHintDescriptor>,
    #[serde(default)]
    fetch_policy_descriptors: Vec<ExpectedFetchPolicyDescriptor>,
    #[serde(default)]
    resource_endpoint_descriptors: Option<Vec<ExpectedResourceEndpointDescriptor>>,
    #[serde(default)]
    link_resource_descriptors: Option<Vec<ExpectedLinkResourceDescriptor>>,
    #[serde(default)]
    form_policy_descriptors: Vec<ExpectedFormPolicyDescriptor>,
    #[serde(default)]
    form_control_descriptors: Option<Vec<ExpectedFormControlDescriptor>>,
    #[serde(default)]
    form_association_descriptors: Option<Vec<ExpectedFormAssociationDescriptor>>,
    #[serde(default)]
    form_autofill_descriptors: Option<Vec<ExpectedFormAutofillDescriptor>>,
    #[serde(default)]
    form_submission_descriptors: Option<Vec<ExpectedFormSubmissionDescriptor>>,
    #[serde(default)]
    form_reset_descriptors: Option<Vec<ExpectedFormResetDescriptor>>,
    #[serde(default)]
    form_validation_descriptors: Option<Vec<ExpectedFormValidationDescriptor>>,
    anchors: Vec<ExpectedAnchor>,
    #[serde(default)]
    anchor_descriptors: Option<Vec<ExpectedAnchorDescriptor>>,
    headings: Vec<ExpectedHeading>,
    #[serde(default)]
    heading_descriptors: Option<Vec<ExpectedHeadingDescriptor>>,
    #[serde(default)]
    text_semantics: Vec<ExpectedTextSemantic>,
    #[serde(default)]
    text_semantic_descriptors: Option<Vec<ExpectedTextSemanticDescriptor>>,
    #[serde(default)]
    text_flow_descriptors: Option<Vec<ExpectedTextFlowDescriptor>>,
    #[serde(default)]
    navigation_target_descriptors: Vec<ExpectedNavigationTargetDescriptor>,
    #[serde(default)]
    navigation_groups: Vec<ExpectedNavigationGroup>,
    #[serde(default)]
    navigation_group_descriptors: Option<Vec<ExpectedNavigationGroupDescriptor>>,
    #[serde(default)]
    section_landmarks: Vec<ExpectedSectionLandmark>,
    #[serde(default)]
    section_landmark_descriptors: Option<Vec<ExpectedSectionLandmarkDescriptor>>,
    #[serde(default)]
    command_elements: Vec<ExpectedCommandElement>,
    #[serde(default)]
    activation_descriptors: Option<Vec<ExpectedActivationDescriptor>>,
    #[serde(default)]
    popovers: Vec<ExpectedPopover>,
    #[serde(default)]
    popover_descriptors: Option<Vec<ExpectedPopoverDescriptor>>,
    #[serde(default)]
    aria_collections: Vec<ExpectedAriaCollection>,
    #[serde(default)]
    aria_collection_descriptors: Option<Vec<ExpectedAriaCollectionDescriptor>>,
    #[serde(default)]
    aria_ranges: Vec<ExpectedAriaRange>,
    #[serde(default)]
    aria_live_regions: Vec<ExpectedAriaLiveRegion>,
    #[serde(default)]
    aria_name_descriptors: Option<Vec<ExpectedAriaNameDescriptor>>,
    #[serde(default)]
    aria_description_descriptors: Option<Vec<ExpectedAriaDescriptionDescriptor>>,
    #[serde(default)]
    aria_relation_descriptors: Vec<ExpectedAriaRelationDescriptor>,
    links: Vec<ExpectedLink>,
    images: Vec<ExpectedImage>,
    #[serde(default)]
    image_candidate_descriptors: Option<Vec<ExpectedImageCandidateDescriptor>>,
    #[serde(default)]
    image_maps: Vec<ExpectedImageMap>,
    #[serde(default)]
    image_map_descriptors: Option<Vec<ExpectedImageMapDescriptor>>,
    #[serde(default)]
    media: Vec<ExpectedMedia>,
    #[serde(default)]
    media_playback_descriptors: Option<Vec<ExpectedMediaPlaybackDescriptor>>,
    #[serde(default)]
    media_resource_descriptors: Option<Vec<ExpectedMediaResourceDescriptor>>,
    #[serde(default)]
    embedded_contexts: Vec<ExpectedEmbeddedContext>,
    #[serde(default)]
    embedded_policy_descriptors: Option<Vec<ExpectedEmbeddedPolicyDescriptor>>,
    #[serde(default)]
    interactive_elements: Vec<ExpectedInteractiveElement>,
    #[serde(default)]
    focus_navigation_descriptors: Option<Vec<ExpectedFocusNavigationDescriptor>>,
    #[serde(default)]
    keyboard_interaction_descriptors: Option<Vec<ExpectedKeyboardInteractionDescriptor>>,
    #[serde(default)]
    input_planning_descriptors: Option<Vec<ExpectedInputPlanningDescriptor>>,
    #[serde(default)]
    drag_drop_descriptors: Option<Vec<ExpectedDragDropDescriptor>>,
    #[serde(default)]
    clipboard_interaction_descriptors: Option<Vec<ExpectedClipboardInteractionDescriptor>>,
    #[serde(default)]
    selection_interaction_descriptors: Option<Vec<ExpectedSelectionInteractionDescriptor>>,
    #[serde(default)]
    composition_interaction_descriptors: Option<Vec<ExpectedCompositionInteractionDescriptor>>,
    #[serde(default)]
    pointer_interaction_descriptors: Option<Vec<ExpectedPointerInteractionDescriptor>>,
    #[serde(default)]
    scroll_interaction_descriptors: Option<Vec<ExpectedScrollInteractionDescriptor>>,
    #[serde(default)]
    disclosures: Vec<ExpectedDisclosure>,
    #[serde(default)]
    disclosure_state_descriptors: Option<Vec<ExpectedDisclosureStateDescriptor>>,
    #[serde(default)]
    template_descriptors: Option<Vec<ExpectedTemplateDescriptor>>,
    #[serde(default)]
    slot_descriptors: Option<Vec<ExpectedSlotDescriptor>>,
    #[serde(default)]
    custom_element_descriptors: Option<Vec<ExpectedCustomElementDescriptor>>,
    #[serde(default)]
    canvas_descriptors: Option<Vec<ExpectedCanvasDescriptor>>,
    #[serde(default)]
    component_hydration_targets: Vec<ExpectedComponentHydrationTarget>,
    #[serde(default)]
    component_hydration_descriptors: Option<Vec<ExpectedComponentHydrationDescriptor>>,
    #[serde(default)]
    data_attribute_descriptors: Vec<ExpectedDataAttributeDescriptor>,
    #[serde(default)]
    global_state_descriptors: Vec<ExpectedGlobalStateDescriptor>,
    #[serde(default)]
    structured_data_descriptors: Option<Vec<ExpectedStructuredDataDescriptor>>,
    #[serde(default)]
    structured_items: Vec<ExpectedStructuredItem>,
    #[serde(default)]
    templates: Vec<ExpectedTemplate>,
    forms: Vec<ExpectedForm>,
    tables: Vec<ExpectedTable>,
    #[serde(default)]
    table_cells: Vec<ExpectedTableCell>,
    #[serde(default)]
    table_structure_descriptors: Option<Vec<ExpectedTableStructureDescriptor>>,
}

#[derive(Debug, Default, Deserialize)]
struct ExpectedDocumentMetadata {
    #[serde(default)]
    charset: Option<String>,
    #[serde(default)]
    viewport: Option<String>,
    #[serde(default)]
    viewport_directives: Vec<ExpectedMetadataDirective>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    application_name: Option<String>,
    #[serde(default)]
    referrer_policy: Option<String>,
    #[serde(default)]
    robots: Option<String>,
    #[serde(default)]
    robots_directives: Vec<String>,
    #[serde(default)]
    color_scheme: Option<String>,
    #[serde(default)]
    http_equiv_hints: Vec<ExpectedHttpEquivHint>,
    #[serde(default)]
    resource_hints: Vec<ExpectedResourceHint>,
    #[serde(default)]
    theme_colors: Vec<ExpectedThemeColor>,
    #[serde(default)]
    refresh: Option<ExpectedRefresh>,
    #[serde(default)]
    canonical_url: Option<String>,
    #[serde(default)]
    resolved_canonical_url: Option<String>,
    #[serde(default)]
    manifest_url: Option<String>,
    #[serde(default)]
    resolved_manifest_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMetadataDirective {
    name: String,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedHttpEquivHint {
    name: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedResourceHint {
    kind: String,
    url: String,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    as_hint: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    imagesrcset: Option<String>,
    #[serde(default)]
    resolved_imagesrcset: Option<String>,
    #[serde(default)]
    imagesizes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedThemeColor {
    color: String,
    #[serde(default)]
    media: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedRefresh {
    #[serde(default)]
    delay: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    resolved_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMeta {
    name: Option<String>,
    http_equiv: Option<String>,
    property: Option<String>,
    charset: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStructuredItem {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    item_type: Vec<String>,
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    resolved_item_id: Option<String>,
    #[serde(default)]
    item_ref: Vec<String>,
    #[serde(default)]
    properties: Vec<ExpectedStructuredProperty>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStructuredProperty {
    name: String,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    value_url: Option<String>,
    #[serde(default)]
    resolved_value_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStructuredDataDescriptor {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    item_type: Vec<String>,
    #[serde(default)]
    item_type_count: usize,
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    resolved_item_id: Option<String>,
    #[serde(default)]
    item_ref: Vec<String>,
    #[serde(default)]
    item_ref_count: usize,
    #[serde(default)]
    unresolved_item_refs: Vec<String>,
    #[serde(default)]
    property_names: Vec<String>,
    #[serde(default)]
    property_count: usize,
    #[serde(default)]
    url_property_count: usize,
    #[serde(default)]
    structured_data_blocked: bool,
    #[serde(default)]
    structured_data_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTemplate {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    shadowrootmode: Option<String>,
    #[serde(default)]
    shadowrootdelegatesfocus: bool,
    #[serde(default)]
    shadowrootclonable: bool,
    #[serde(default)]
    shadowrootserializable: bool,
    content_text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedTemplateDescriptor {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    template_kind: String,
    #[serde(default)]
    shadowrootmode: Option<String>,
    #[serde(default)]
    shadowroot_attribute_names: Vec<String>,
    #[serde(default)]
    shadowroot_attribute_count: usize,
    #[serde(default)]
    declarative_shadow_root: bool,
    #[serde(default)]
    shadowroot_mode_valid: bool,
    #[serde(default)]
    shadowrootdelegatesfocus: bool,
    #[serde(default)]
    shadowrootclonable: bool,
    #[serde(default)]
    shadowrootserializable: bool,
    #[serde(default)]
    content_text: String,
    #[serde(default)]
    content_word_count: usize,
    #[serde(default)]
    template_blocked: bool,
    #[serde(default)]
    template_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedSlotDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    slot_kind: String,
    #[serde(default)]
    slot: Option<String>,
    #[serde(default)]
    slot_name: Option<String>,
    #[serde(default)]
    default_slot: bool,
    #[serde(default)]
    named_slot: bool,
    #[serde(default)]
    fallback_text: String,
    #[serde(default)]
    fallback_word_count: usize,
    #[serde(default)]
    part: Vec<String>,
    #[serde(default)]
    custom_element: bool,
    #[serde(default)]
    custom_element_name: Option<String>,
    #[serde(default)]
    custom_element_is: Option<String>,
    #[serde(default)]
    slot_blocked: bool,
    #[serde(default)]
    slot_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCustomElementDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    custom_element_kind: String,
    #[serde(default)]
    definition_name: Option<String>,
    #[serde(default)]
    custom_element_name: Option<String>,
    #[serde(default)]
    custom_element_is: Option<String>,
    #[serde(default)]
    autonomous_custom_element: bool,
    #[serde(default)]
    customized_builtin: bool,
    #[serde(default)]
    extends_element: Option<String>,
    #[serde(default)]
    custom_element_name_valid: bool,
    #[serde(default)]
    slot: Option<String>,
    #[serde(default)]
    part: Vec<String>,
    #[serde(default)]
    exportparts: Option<String>,
    #[serde(default)]
    data_attribute_names: Vec<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    custom_element_blocked: bool,
    #[serde(default)]
    custom_element_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCanvasDescriptor {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    has_width: bool,
    #[serde(default)]
    has_height: bool,
    #[serde(default)]
    fallback_text: String,
    #[serde(default)]
    fallback_word_count: usize,
    #[serde(default)]
    part: Vec<String>,
    #[serde(default)]
    data_attribute_names: Vec<String>,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    pointer_handlers: Vec<String>,
    #[serde(default)]
    keyboard_handlers: Vec<String>,
    #[serde(default)]
    lifecycle_handlers: Vec<String>,
    #[serde(default)]
    canvas_blocked: bool,
    #[serde(default)]
    canvas_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedComponentHydrationTarget {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    custom_element: bool,
    #[serde(default)]
    custom_element_name: Option<String>,
    #[serde(default)]
    custom_element_is: Option<String>,
    #[serde(default)]
    slot: Option<String>,
    #[serde(default)]
    slot_name: Option<String>,
    #[serde(default)]
    part: Vec<String>,
    #[serde(default)]
    exportparts: Option<String>,
    #[serde(default)]
    data_attributes: Vec<ExpectedDataAttribute>,
    #[serde(default)]
    canvas_fallback_text: Option<String>,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedComponentHydrationDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    hydration_kind: String,
    #[serde(default)]
    custom_element: bool,
    #[serde(default)]
    custom_element_name: Option<String>,
    #[serde(default)]
    custom_element_is: Option<String>,
    #[serde(default)]
    shadowrootmode: Option<String>,
    #[serde(default)]
    slot: Option<String>,
    #[serde(default)]
    slot_name: Option<String>,
    #[serde(default)]
    part: Vec<String>,
    #[serde(default)]
    exportparts: Option<String>,
    #[serde(default)]
    data_attribute_names: Vec<String>,
    #[serde(default)]
    data_attribute_count: usize,
    #[serde(default)]
    canvas_fallback_text: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    hydration_blocked: bool,
    #[serde(default)]
    hydration_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDataAttribute {
    name: String,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDataAttributeDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    custom_element: bool,
    #[serde(default)]
    custom_element_name: Option<String>,
    #[serde(default)]
    custom_element_is: Option<String>,
    #[serde(default)]
    slot: Option<String>,
    #[serde(default)]
    slot_name: Option<String>,
    #[serde(default)]
    part: Vec<String>,
    #[serde(default)]
    data_attributes: Vec<ExpectedDataAttribute>,
    #[serde(default)]
    data_attribute_names: Vec<String>,
    #[serde(default)]
    data_attribute_count: usize,
    #[serde(default)]
    json_data_attribute_names: Vec<String>,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedGlobalStateDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    dir: Option<String>,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    autofocus: bool,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    draggable: Option<String>,
    #[serde(default)]
    draggable_state: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    translate: Option<String>,
    #[serde(default)]
    global_attribute_names: Vec<String>,
    #[serde(default)]
    global_attribute_count: usize,
    #[serde(default)]
    focus_navigation_hint: bool,
    #[serde(default)]
    global_state_blocked: bool,
    #[serde(default)]
    global_state_block_reasons: Vec<String>,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaRelationDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    aria_details: Vec<String>,
    #[serde(default)]
    details_text: Vec<String>,
    #[serde(default)]
    aria_errormessage: Vec<String>,
    #[serde(default)]
    errormessage_text: Vec<String>,
    #[serde(default)]
    aria_flowto: Vec<String>,
    #[serde(default)]
    flowto_text: Vec<String>,
    #[serde(default)]
    relation_attribute_names: Vec<String>,
    #[serde(default)]
    relation_attribute_count: usize,
    #[serde(default)]
    relation_target_count: usize,
    #[serde(default)]
    unresolved_relation_targets: Vec<String>,
    #[serde(default)]
    relation_blocked: bool,
    #[serde(default)]
    relation_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaNameDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    labelledby_text: Vec<String>,
    #[serde(default)]
    name_source: String,
    #[serde(default)]
    name_attribute_names: Vec<String>,
    #[serde(default)]
    name_attribute_count: usize,
    #[serde(default)]
    label_target_count: usize,
    #[serde(default)]
    unresolved_label_targets: Vec<String>,
    #[serde(default)]
    name_blocked: bool,
    #[serde(default)]
    name_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaDescriptionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_description: Option<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    #[serde(default)]
    describedby_text: Vec<String>,
    #[serde(default)]
    description_source: String,
    #[serde(default)]
    description_attribute_names: Vec<String>,
    #[serde(default)]
    description_attribute_count: usize,
    #[serde(default)]
    description_target_count: usize,
    #[serde(default)]
    unresolved_description_targets: Vec<String>,
    #[serde(default)]
    description_blocked: bool,
    #[serde(default)]
    description_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDocumentPolicyDescriptor {
    #[serde(default)]
    charset: Option<String>,
    #[serde(default)]
    viewport: Option<String>,
    #[serde(default)]
    viewport_directives: Vec<ExpectedMetadataDirective>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    application_name: Option<String>,
    #[serde(default)]
    referrer_policy: Option<String>,
    #[serde(default)]
    robots: Option<String>,
    #[serde(default)]
    robots_directives: Vec<String>,
    #[serde(default)]
    color_scheme: Option<String>,
    #[serde(default)]
    content_security_policy: Option<String>,
    #[serde(default)]
    permissions_policy: Option<String>,
    #[serde(default)]
    permissions_policy_features: Vec<String>,
    #[serde(default)]
    permissions_policy_feature_count: usize,
    #[serde(default)]
    origin_trials: Vec<String>,
    #[serde(default)]
    accept_ch: Option<String>,
    #[serde(default)]
    accept_ch_tokens: Vec<String>,
    #[serde(default)]
    dns_prefetch_control: Option<String>,
    #[serde(default)]
    theme_colors: Vec<ExpectedThemeColor>,
    #[serde(default)]
    refresh: Option<ExpectedRefresh>,
    #[serde(default)]
    canonical_url: Option<String>,
    #[serde(default)]
    resolved_canonical_url: Option<String>,
    #[serde(default)]
    manifest_url: Option<String>,
    #[serde(default)]
    resolved_manifest_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedNavigationTargetDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    rel_external: bool,
    #[serde(default)]
    rel_nofollow: bool,
    #[serde(default)]
    rel_noopener: bool,
    #[serde(default)]
    rel_noreferrer: bool,
    #[serde(default)]
    download: Option<String>,
    #[serde(default)]
    ping: Vec<String>,
    #[serde(default)]
    resolved_ping: Vec<String>,
    #[serde(default)]
    attributionsrc: Vec<String>,
    #[serde(default)]
    resolved_attributionsrc: Vec<String>,
    #[serde(default)]
    hreflang: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    area_shape: Option<String>,
    #[serde(default)]
    area_coords: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedLoadingHintDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    decoding: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    preload: Option<String>,
    #[serde(default)]
    as_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFetchPolicyDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    csp: Option<String>,
    #[serde(default)]
    sandbox: Vec<String>,
    #[serde(default)]
    allow: Option<String>,
    #[serde(default)]
    allowfullscreen: bool,
    #[serde(default)]
    credentialless: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedEventHandlerDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    role: Option<String>,
    source: String,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    activation_handlers: Vec<String>,
    #[serde(default)]
    keyboard_handlers: Vec<String>,
    #[serde(default)]
    pointer_handlers: Vec<String>,
    #[serde(default)]
    form_handlers: Vec<String>,
    #[serde(default)]
    media_handlers: Vec<String>,
    #[serde(default)]
    lifecycle_handlers: Vec<String>,
    #[serde(default)]
    error_handlers: Vec<String>,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedLifecycleEventDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    role: Option<String>,
    source: String,
    lifecycle_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    lifecycle_handlers: Vec<String>,
    #[serde(default)]
    load_handlers: Vec<String>,
    #[serde(default)]
    unload_handlers: Vec<String>,
    #[serde(default)]
    visibility_handlers: Vec<String>,
    #[serde(default)]
    history_handlers: Vec<String>,
    #[serde(default)]
    network_handlers: Vec<String>,
    #[serde(default)]
    error_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    document_scope: bool,
    #[serde(default)]
    body_scope: bool,
    #[serde(default)]
    error_recovery: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedAnimationInteractionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    role: Option<String>,
    source: String,
    animation_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    animation_handlers: Vec<String>,
    #[serde(default)]
    animation_start_handlers: Vec<String>,
    #[serde(default)]
    animation_iteration_handlers: Vec<String>,
    #[serde(default)]
    animation_end_handlers: Vec<String>,
    #[serde(default)]
    animation_cancel_handlers: Vec<String>,
    #[serde(default)]
    transition_handlers: Vec<String>,
    #[serde(default)]
    transition_run_handlers: Vec<String>,
    #[serde(default)]
    transition_start_handlers: Vec<String>,
    #[serde(default)]
    transition_end_handlers: Vec<String>,
    #[serde(default)]
    transition_cancel_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    document_scope: bool,
    #[serde(default)]
    body_scope: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedFullscreenInteractionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    role: Option<String>,
    source: String,
    fullscreen_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    fullscreen_handlers: Vec<String>,
    #[serde(default)]
    fullscreen_change_handlers: Vec<String>,
    #[serde(default)]
    fullscreen_error_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    allow: Option<String>,
    #[serde(default)]
    allow_tokens: Vec<String>,
    #[serde(default)]
    allowfullscreen: bool,
    #[serde(default)]
    fullscreen_allowed: bool,
    #[serde(default)]
    embedded_context: bool,
    #[serde(default)]
    document_scope: bool,
    #[serde(default)]
    body_scope: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedContextMenuInteractionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    source: String,
    context_menu_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    popover: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    contextmenu_handlers: Vec<String>,
    #[serde(default)]
    pointer_handlers: Vec<String>,
    #[serde(default)]
    keyboard_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    context_menu_blocked: bool,
    #[serde(default)]
    context_menu_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormPolicyDescriptor {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    resolved_action: Option<String>,
    method: String,
    #[serde(default)]
    enctype: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    accept_charset: Option<String>,
    #[serde(default)]
    accept_charset_tokens: Vec<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    rel_external: bool,
    #[serde(default)]
    rel_nofollow: bool,
    #[serde(default)]
    rel_noopener: bool,
    #[serde(default)]
    rel_noreferrer: bool,
    #[serde(default)]
    novalidate: bool,
    #[serde(default)]
    submitters: Vec<ExpectedFormPolicySubmitterDescriptor>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormPolicySubmitterDescriptor {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    resolved_action: Option<String>,
    method: String,
    #[serde(default)]
    enctype: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    novalidate: bool,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormControlDescriptor {
    #[serde(default)]
    form_id: Option<String>,
    #[serde(default)]
    form_name: Option<String>,
    element: String,
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    control_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    label_count: usize,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    submission_value_count: usize,
    #[serde(default)]
    placeholder: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    #[serde(default)]
    datalist_options: Vec<String>,
    #[serde(default)]
    option_count: usize,
    #[serde(default)]
    selected_options: Vec<String>,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    autofocus: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
    #[serde(default)]
    fieldset_ids: Vec<String>,
    #[serde(default)]
    fieldset_legends: Vec<String>,
    #[serde(default)]
    control_blocked: bool,
    #[serde(default)]
    control_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormAssociationDescriptor {
    #[serde(default)]
    form_id: Option<String>,
    #[serde(default)]
    form_name: Option<String>,
    element: String,
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    association_kind: String,
    #[serde(default)]
    explicit_form_owner: bool,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    label_count: usize,
    #[serde(default)]
    fieldset_ids: Vec<String>,
    #[serde(default)]
    fieldset_legends: Vec<String>,
    #[serde(default)]
    datalist_id: Option<String>,
    #[serde(default)]
    datalist_option_count: usize,
    #[serde(default)]
    output_for_tokens: Vec<String>,
    #[serde(default)]
    output_target_ids: Vec<String>,
    #[serde(default)]
    output_target_names: Vec<String>,
    #[serde(default)]
    output_target_types: Vec<String>,
    #[serde(default)]
    referenced_by_output_ids: Vec<String>,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormAutofillDescriptor {
    #[serde(default)]
    form_id: Option<String>,
    #[serde(default)]
    form_name: Option<String>,
    #[serde(default)]
    form_autocomplete: Option<String>,
    #[serde(default)]
    form_autocomplete_tokens: Vec<String>,
    #[serde(default)]
    form_autocomplete_enabled: bool,
    element: String,
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    autofill_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    #[serde(default)]
    autocomplete_token_count: usize,
    #[serde(default)]
    section_token: Option<String>,
    #[serde(default)]
    address_type_token: Option<String>,
    #[serde(default)]
    contact_type_token: Option<String>,
    #[serde(default)]
    field_token: Option<String>,
    #[serde(default)]
    webauthn: bool,
    #[serde(default)]
    autofill_enabled: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    autofill_blocked: bool,
    #[serde(default)]
    autofill_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormSubmissionDescriptor {
    #[serde(default)]
    form_id: Option<String>,
    #[serde(default)]
    form_name: Option<String>,
    #[serde(default)]
    form_action: Option<String>,
    #[serde(default)]
    resolved_form_action: Option<String>,
    form_method: String,
    #[serde(default)]
    form_enctype: Option<String>,
    #[serde(default)]
    form_target: Option<String>,
    #[serde(default)]
    effective_form_target: Option<String>,
    element: String,
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    submission_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    submission_value_count: usize,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    submitter: bool,
    #[serde(default)]
    submitter_action: Option<String>,
    #[serde(default)]
    resolved_submitter_action: Option<String>,
    #[serde(default)]
    submitter_method: Option<String>,
    #[serde(default)]
    submitter_enctype: Option<String>,
    #[serde(default)]
    submitter_target: Option<String>,
    #[serde(default)]
    effective_submitter_target: Option<String>,
    #[serde(default)]
    submitter_novalidate: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormResetDescriptor {
    #[serde(default)]
    form_id: Option<String>,
    #[serde(default)]
    form_name: Option<String>,
    #[serde(default)]
    form_autocomplete: Option<String>,
    #[serde(default)]
    form_event_handlers: Vec<String>,
    #[serde(default)]
    form_reset_handlers: Vec<String>,
    #[serde(default)]
    form_has_reset_handler: bool,
    element: String,
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    reset_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    reset_values: Vec<String>,
    #[serde(default)]
    reset_value_count: usize,
    #[serde(default)]
    selected_options: Vec<String>,
    #[serde(default)]
    option_count: usize,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    resettable: bool,
    #[serde(default)]
    resetter: bool,
    #[serde(default)]
    reset_blocked: bool,
    #[serde(default)]
    reset_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormValidationDescriptor {
    #[serde(default)]
    form_id: Option<String>,
    #[serde(default)]
    form_name: Option<String>,
    #[serde(default)]
    form_novalidate: bool,
    element: String,
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    validation_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_attribute_count: usize,
    #[serde(default)]
    validation_barred_reason: Option<String>,
    #[serde(default)]
    validation_blocked: bool,
    #[serde(default)]
    validation_block_reasons: Vec<String>,
    #[serde(default)]
    submitter_ids: Vec<String>,
    #[serde(default)]
    submitter_novalidate_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedResource {
    kind: String,
    url: String,
    resolved_url: Option<String>,
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    as_hint: Option<String>,
    type_hint: Option<String>,
    media: Option<String>,
    title: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    hreflang: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    csp: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    browsing_context_name: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    sandbox: Vec<String>,
    #[serde(default)]
    allow: Option<String>,
    #[serde(default)]
    allowfullscreen: bool,
    #[serde(default)]
    srcdoc: Option<String>,
    #[serde(default)]
    credentialless: bool,
    #[serde(default)]
    imagesrcset: Option<String>,
    #[serde(default)]
    resolved_imagesrcset: Option<String>,
    #[serde(default)]
    imagesizes: Option<String>,
    #[serde(default)]
    track_kind: Option<String>,
    #[serde(default)]
    srclang: Option<String>,
    #[serde(default)]
    track_label: Option<String>,
    #[serde(default)]
    default_track: bool,
    async_script: bool,
    defer_script: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedLinkResourceDescriptor {
    resource_index: usize,
    resource_kind: String,
    url: String,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    as_hint: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    hreflang: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    responsive_image_preload: bool,
    #[serde(default)]
    icon_candidate: bool,
    #[serde(default)]
    alternate_candidate: bool,
    #[serde(default)]
    policy_hint_count: usize,
    #[serde(default)]
    resource_blocked: bool,
    #[serde(default)]
    resource_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedResourceEndpointDescriptor {
    endpoint_kind: String,
    element: String,
    #[serde(default)]
    resource_kind: Option<String>,
    url: String,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    as_hint: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    hreflang: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    csp: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    browsing_context_name: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    sandbox: Vec<String>,
    #[serde(default)]
    allow: Option<String>,
    #[serde(default)]
    allowfullscreen: bool,
    #[serde(default)]
    srcdoc: Option<String>,
    #[serde(default)]
    credentialless: bool,
    #[serde(default)]
    imagesrcset: Option<String>,
    #[serde(default)]
    resolved_imagesrcset: Option<String>,
    #[serde(default)]
    imagesizes: Option<String>,
    #[serde(default)]
    track_kind: Option<String>,
    #[serde(default)]
    srclang: Option<String>,
    #[serde(default)]
    track_label: Option<String>,
    #[serde(default)]
    default_track: bool,
    #[serde(default)]
    async_script: bool,
    #[serde(default)]
    defer_script: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedScript {
    script_kind: String,
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    type_hint: Option<String>,
    #[serde(default)]
    async_script: bool,
    #[serde(default)]
    defer_script: bool,
    #[serde(default)]
    nomodule: bool,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedScriptExecutionDescriptor {
    script_kind: String,
    #[serde(default)]
    execution_kind: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    async_script: bool,
    #[serde(default)]
    defer_script: bool,
    #[serde(default)]
    nomodule: bool,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    blocking_token_count: usize,
    #[serde(default)]
    render_blocking: bool,
    #[serde(default)]
    has_text: bool,
    #[serde(default)]
    text_length: usize,
}

#[derive(Debug, Deserialize)]
struct ExpectedScriptStorageAccessDescriptor {
    script_kind: String,
    access_kind: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    execution_kind: String,
    #[serde(default)]
    has_text: bool,
    #[serde(default)]
    text_length: usize,
    #[serde(default)]
    storage_targets: Vec<String>,
    #[serde(default)]
    storage_target_count: usize,
    #[serde(default)]
    uses_local_storage: bool,
    #[serde(default)]
    uses_session_storage: bool,
    #[serde(default)]
    uses_cookies: bool,
    #[serde(default)]
    uses_indexed_db: bool,
    #[serde(default)]
    uses_cache_storage: bool,
    #[serde(default)]
    uses_service_worker: bool,
    #[serde(default)]
    uses_storage_manager: bool,
    #[serde(default)]
    listens_storage_events: bool,
    #[serde(default)]
    storage_blocked: bool,
    #[serde(default)]
    storage_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedScriptWorkerMessagingDescriptor {
    script_kind: String,
    messaging_kind: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    execution_kind: String,
    #[serde(default)]
    has_text: bool,
    #[serde(default)]
    text_length: usize,
    #[serde(default)]
    messaging_targets: Vec<String>,
    #[serde(default)]
    messaging_target_count: usize,
    #[serde(default)]
    creates_worker: bool,
    #[serde(default)]
    creates_shared_worker: bool,
    #[serde(default)]
    registers_service_worker: bool,
    #[serde(default)]
    uses_post_message: bool,
    #[serde(default)]
    listens_message_events: bool,
    #[serde(default)]
    uses_message_channel: bool,
    #[serde(default)]
    uses_broadcast_channel: bool,
    #[serde(default)]
    uses_import_scripts: bool,
    #[serde(default)]
    module_worker_hint: bool,
    #[serde(default)]
    messaging_blocked: bool,
    #[serde(default)]
    messaging_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedScriptModuleGraphDescriptor {
    script_kind: String,
    module_graph_kind: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    execution_kind: String,
    #[serde(default)]
    has_text: bool,
    #[serde(default)]
    text_length: usize,
    #[serde(default)]
    module_targets: Vec<String>,
    #[serde(default)]
    module_target_count: usize,
    #[serde(default)]
    external_module_entry: bool,
    #[serde(default)]
    inline_module_entry: bool,
    #[serde(default)]
    declares_import_map: bool,
    #[serde(default)]
    uses_static_imports: bool,
    #[serde(default)]
    uses_dynamic_imports: bool,
    #[serde(default)]
    has_modulepreload: bool,
    #[serde(default)]
    modulepreload_urls: Vec<String>,
    #[serde(default)]
    resolved_modulepreload_urls: Vec<String>,
    #[serde(default)]
    module_graph_blocked: bool,
    #[serde(default)]
    module_graph_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStylesheet {
    source: String,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    alternate: bool,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStylesheetPlanningDescriptor {
    source: String,
    stylesheet_kind: String,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    rel_token_count: usize,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    alternate: bool,
    #[serde(default)]
    applies_by_default: bool,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    blocking: Option<String>,
    #[serde(default)]
    blocking_tokens: Vec<String>,
    #[serde(default)]
    blocking_token_count: usize,
    #[serde(default)]
    render_blocking: bool,
    #[serde(default)]
    has_text: bool,
    #[serde(default)]
    text_length: usize,
}

#[derive(Debug, Deserialize)]
struct ExpectedAnchor {
    id: Option<String>,
    name: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedAnchorDescriptor {
    anchor_index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    fragment_targets: Vec<String>,
    anchor_kind: String,
    #[serde(default)]
    duplicate_target: bool,
    #[serde(default)]
    anchor_blocked: bool,
    #[serde(default)]
    anchor_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedHeading {
    level: u8,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedHeadingDescriptor {
    heading_index: usize,
    level: u8,
    #[serde(default)]
    text: String,
    #[serde(default)]
    previous_level: Option<u8>,
    outline_kind: String,
    #[serde(default)]
    skipped_level: bool,
    #[serde(default)]
    heading_blocked: bool,
    #[serde(default)]
    heading_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTextSemantic {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    dir: Option<String>,
    #[serde(default)]
    quote_cite: Option<String>,
    #[serde(default)]
    resolved_quote_cite: Option<String>,
    #[serde(default)]
    data_value: Option<String>,
    #[serde(default)]
    datetime: Option<String>,
    #[serde(default)]
    edit_cite: Option<String>,
    #[serde(default)]
    resolved_edit_cite: Option<String>,
    #[serde(default)]
    edit_datetime: Option<String>,
    #[serde(default)]
    ruby_kind: Option<String>,
    #[serde(default)]
    bidi_kind: Option<String>,
    #[serde(default)]
    phrase_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTextSemanticDescriptor {
    semantic_index: usize,
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    semantic_kind: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    dir: Option<String>,
    #[serde(default)]
    quote_cite: Option<String>,
    #[serde(default)]
    resolved_quote_cite: Option<String>,
    #[serde(default)]
    data_value: Option<String>,
    #[serde(default)]
    datetime: Option<String>,
    #[serde(default)]
    edit_cite: Option<String>,
    #[serde(default)]
    resolved_edit_cite: Option<String>,
    #[serde(default)]
    edit_datetime: Option<String>,
    #[serde(default)]
    ruby_kind: Option<String>,
    #[serde(default)]
    bidi_kind: Option<String>,
    #[serde(default)]
    phrase_kind: Option<String>,
    #[serde(default)]
    semantic_blocked: bool,
    #[serde(default)]
    semantic_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTextFlowDescriptor {
    flow_index: usize,
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    flow_kind: String,
    #[serde(default)]
    text_flow: Option<String>,
    #[serde(default)]
    list_kind: Option<String>,
    #[serde(default)]
    list_start: Option<String>,
    #[serde(default)]
    list_marker_type: Option<String>,
    #[serde(default)]
    list_reversed: bool,
    #[serde(default)]
    list_item_value: Option<String>,
    #[serde(default)]
    list_item_count: usize,
    #[serde(default)]
    description_list_kind: Option<String>,
    #[serde(default)]
    term_kind: Option<String>,
    #[serde(default)]
    term_count: usize,
    #[serde(default)]
    description_count: usize,
    #[serde(default)]
    quote_cite: Option<String>,
    #[serde(default)]
    resolved_quote_cite: Option<String>,
    #[serde(default)]
    flow_blocked: bool,
    #[serde(default)]
    flow_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedNavigationGroup {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    landmark_kind: Option<String>,
    #[serde(default)]
    list_kind: Option<String>,
    item_count: usize,
    #[serde(default)]
    list_start: Option<String>,
    #[serde(default)]
    list_marker_type: Option<String>,
    #[serde(default)]
    list_reversed: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedNavigationGroupDescriptor {
    group_index: usize,
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    group_kind: String,
    #[serde(default)]
    landmark_kind: Option<String>,
    #[serde(default)]
    list_kind: Option<String>,
    item_count: usize,
    #[serde(default)]
    list_start: Option<String>,
    #[serde(default)]
    list_marker_type: Option<String>,
    #[serde(default)]
    list_reversed: bool,
    #[serde(default)]
    navigation_blocked: bool,
    #[serde(default)]
    navigation_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedSectionLandmark {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    #[serde(default)]
    authored_role: Option<String>,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    section_kind: Option<String>,
    #[serde(default)]
    landmark_kind: Option<String>,
    #[serde(default)]
    heading_level: Option<u8>,
    #[serde(default)]
    heading_text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedSectionLandmarkDescriptor {
    landmark_index: usize,
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    #[serde(default)]
    authored_role: Option<String>,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    section_kind: Option<String>,
    #[serde(default)]
    landmark_kind: Option<String>,
    #[serde(default)]
    heading_level: Option<u8>,
    #[serde(default)]
    heading_text: Option<String>,
    outline_kind: String,
    #[serde(default)]
    landmark_blocked: bool,
    #[serde(default)]
    landmark_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCommandElement {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    #[serde(default)]
    authored_role: Option<String>,
    command_kind: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    form_action: Option<String>,
    #[serde(default)]
    resolved_form_action: Option<String>,
    #[serde(default)]
    form_method: Option<String>,
    #[serde(default)]
    form_target: Option<String>,
    #[serde(default)]
    form_novalidate: bool,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_pressed: Option<String>,
    #[serde(default)]
    aria_current: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedActivationDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    #[serde(default)]
    authored_role: Option<String>,
    command_kind: String,
    activation_kind: String,
    #[serde(default)]
    target_id: Option<String>,
    target_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_pressed: Option<String>,
    #[serde(default)]
    aria_current: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    form_action: Option<String>,
    #[serde(default)]
    resolved_form_action: Option<String>,
    #[serde(default)]
    form_method: Option<String>,
    #[serde(default)]
    form_target: Option<String>,
    #[serde(default)]
    form_novalidate: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedPopover {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    popover: String,
    #[serde(default)]
    invokers: Vec<ExpectedPopoverInvoker>,
}

#[derive(Debug, Deserialize)]
struct ExpectedPopoverInvoker {
    element: String,
    #[serde(default)]
    id: Option<String>,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    command_kind: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    focusable: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedPopoverDescriptor {
    popover_index: usize,
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    popover_mode: String,
    #[serde(default)]
    invoker_count: usize,
    #[serde(default)]
    invoker_ids: Vec<String>,
    #[serde(default)]
    invoker_actions: Vec<String>,
    #[serde(default)]
    invoker_aria_expanded: Vec<String>,
    #[serde(default)]
    focusable_invoker_count: usize,
    #[serde(default)]
    popover_blocked: bool,
    #[serde(default)]
    popover_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaCollection {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    #[serde(default)]
    aria_orientation: Option<String>,
    #[serde(default)]
    aria_multiselectable: Option<String>,
    #[serde(default)]
    aria_activedescendant: Option<String>,
    #[serde(default)]
    aria_owns: Vec<String>,
    #[serde(default)]
    item_count: usize,
    #[serde(default)]
    selected_item_count: usize,
    #[serde(default)]
    checked_item_count: usize,
    #[serde(default)]
    current_item_count: usize,
    #[serde(default)]
    disabled_item_count: usize,
    #[serde(default)]
    item_roles: Vec<String>,
    #[serde(default)]
    selection_mode: String,
    #[serde(default)]
    active_descendant_matches_item: bool,
    #[serde(default)]
    items: Vec<ExpectedAriaCollectionItem>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaCollectionItem {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    aria_selected: Option<String>,
    #[serde(default)]
    aria_checked: Option<String>,
    #[serde(default)]
    aria_current: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_level: Option<String>,
    #[serde(default)]
    aria_posinset: Option<String>,
    #[serde(default)]
    aria_setsize: Option<String>,
    #[serde(default)]
    aria_rowindex: Option<String>,
    #[serde(default)]
    aria_colindex: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaCollectionDescriptor {
    collection_index: usize,
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    collection_kind: String,
    #[serde(default)]
    aria_orientation: Option<String>,
    #[serde(default)]
    aria_multiselectable: Option<String>,
    #[serde(default)]
    aria_activedescendant: Option<String>,
    #[serde(default)]
    aria_owns: Vec<String>,
    #[serde(default)]
    item_count: usize,
    #[serde(default)]
    item_roles: Vec<String>,
    #[serde(default)]
    selected_item_count: usize,
    #[serde(default)]
    checked_item_count: usize,
    #[serde(default)]
    current_item_count: usize,
    #[serde(default)]
    disabled_item_count: usize,
    #[serde(default)]
    selection_mode: String,
    #[serde(default)]
    active_descendant_matches_item: bool,
    #[serde(default)]
    collection_blocked: bool,
    #[serde(default)]
    collection_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaRange {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    #[serde(default)]
    aria_valuenow: Option<String>,
    #[serde(default)]
    aria_valuemin: Option<String>,
    #[serde(default)]
    aria_valuemax: Option<String>,
    #[serde(default)]
    aria_valuetext: Option<String>,
    #[serde(default)]
    aria_orientation: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    aria_readonly: Option<String>,
    #[serde(default)]
    aria_required: Option<String>,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    text_value: Option<String>,
    #[serde(default)]
    value_attribute_names: Vec<String>,
    #[serde(default)]
    value_attribute_count: usize,
    #[serde(default)]
    range_value_complete: bool,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    range_blocked: bool,
    #[serde(default)]
    range_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAriaLiveRegion {
    element: String,
    #[serde(default)]
    id: Option<String>,
    role: String,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    #[serde(default)]
    aria_live: Option<String>,
    #[serde(default)]
    aria_busy: Option<String>,
    #[serde(default)]
    aria_atomic: Option<String>,
    #[serde(default)]
    aria_relevant: Vec<String>,
    #[serde(default)]
    aria_hidden: bool,
    update_kind: String,
    #[serde(default)]
    live_attribute_names: Vec<String>,
    #[serde(default)]
    live_attribute_count: usize,
    #[serde(default)]
    assertive_update: bool,
    #[serde(default)]
    live_region_blocked: bool,
    #[serde(default)]
    live_region_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedLink {
    #[serde(default = "default_browser_link_element")]
    element: String,
    #[serde(default)]
    id: Option<String>,
    href: Option<String>,
    resolved_href: Option<String>,
    name: Option<String>,
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    rel_external: bool,
    #[serde(default)]
    rel_nofollow: bool,
    #[serde(default)]
    rel_noopener: bool,
    #[serde(default)]
    rel_noreferrer: bool,
    title: Option<String>,
    #[serde(default)]
    download: Option<String>,
    #[serde(default)]
    ping: Vec<String>,
    #[serde(default)]
    resolved_ping: Vec<String>,
    #[serde(default)]
    attributionsrc: Vec<String>,
    #[serde(default)]
    resolved_attributionsrc: Vec<String>,
    #[serde(default)]
    hreflang: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedImage {
    src: Option<String>,
    resolved_src: Option<String>,
    alt: Option<String>,
    width: Option<String>,
    height: Option<String>,
    #[serde(default)]
    srcset: Option<String>,
    #[serde(default)]
    resolved_srcset: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    decoding: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    usemap: Option<String>,
    #[serde(default)]
    ismap: bool,
    #[serde(default)]
    sources: Vec<ExpectedImageSource>,
}

#[derive(Debug, Deserialize)]
struct ExpectedImageSource {
    #[serde(default)]
    srcset: Option<String>,
    #[serde(default)]
    resolved_srcset: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedImageCandidateDescriptor {
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    srcset: Option<String>,
    #[serde(default)]
    resolved_srcset: Option<String>,
    #[serde(default)]
    sizes: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    decoding: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    usemap: Option<String>,
    #[serde(default)]
    ismap: bool,
    #[serde(default)]
    has_alt: bool,
    #[serde(default)]
    source_count: usize,
    #[serde(default)]
    source_srcset_count: usize,
    #[serde(default)]
    candidate_count: usize,
    #[serde(default)]
    source_type_hints: Vec<String>,
    #[serde(default)]
    source_media: Vec<String>,
    #[serde(default)]
    sources: Vec<ExpectedImageSource>,
}

#[derive(Debug, Deserialize)]
struct ExpectedImageMap {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    areas: Vec<ExpectedImageMapArea>,
}

#[derive(Debug, Deserialize)]
struct ExpectedImageMapArea {
    #[serde(default)]
    id: Option<String>,
    #[serde(default = "default_image_map_area_shape")]
    shape: String,
    #[serde(default)]
    coords: Option<String>,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    resolved_href: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    rel_external: bool,
    #[serde(default)]
    rel_nofollow: bool,
    #[serde(default)]
    rel_noopener: bool,
    #[serde(default)]
    rel_noreferrer: bool,
    #[serde(default)]
    ping: Vec<String>,
    #[serde(default)]
    resolved_ping: Vec<String>,
    #[serde(default)]
    attributionsrc: Vec<String>,
    #[serde(default)]
    resolved_attributionsrc: Vec<String>,
    #[serde(default)]
    download: Option<String>,
    #[serde(default)]
    hreflang: Option<String>,
    #[serde(default)]
    referrerpolicy: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedImageMapDescriptor {
    map_index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    referenced_image_sources: Vec<String>,
    #[serde(default)]
    area_count: usize,
    #[serde(default)]
    navigable_area_count: usize,
    #[serde(default)]
    area_shapes: Vec<String>,
    #[serde(default)]
    missing_alt_area_count: usize,
    #[serde(default)]
    missing_href_area_count: usize,
    #[serde(default)]
    missing_coords_area_count: usize,
    #[serde(default)]
    default_shape_area_count: usize,
    #[serde(default)]
    ping_area_count: usize,
    #[serde(default)]
    attribution_area_count: usize,
    #[serde(default)]
    map_blocked: bool,
    #[serde(default)]
    map_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMedia {
    kind: String,
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    poster: Option<String>,
    #[serde(default)]
    resolved_poster: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    controls: bool,
    #[serde(default)]
    autoplay: bool,
    #[serde(default)]
    loop_media: bool,
    #[serde(default)]
    muted: bool,
    #[serde(default)]
    playsinline: bool,
    #[serde(default)]
    preload: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    controlslist: Option<String>,
    #[serde(default)]
    controlslist_tokens: Vec<String>,
    #[serde(default)]
    disableremoteplayback: bool,
    #[serde(default)]
    disablepictureinpicture: bool,
    #[serde(default)]
    sources: Vec<ExpectedMediaSource>,
    #[serde(default)]
    tracks: Vec<ExpectedMediaTrack>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMediaSource {
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMediaTrack {
    #[serde(default = "default_track_kind")]
    kind: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    srclang: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    default_track: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedMediaPlaybackDescriptor {
    kind: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    poster: Option<String>,
    #[serde(default)]
    resolved_poster: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    controls: bool,
    #[serde(default)]
    autoplay: bool,
    #[serde(default)]
    loop_media: bool,
    #[serde(default)]
    muted: bool,
    #[serde(default)]
    playsinline: bool,
    #[serde(default)]
    preload: Option<String>,
    #[serde(default)]
    crossorigin: Option<String>,
    #[serde(default)]
    controlslist: Option<String>,
    #[serde(default)]
    controlslist_tokens: Vec<String>,
    #[serde(default)]
    disableremoteplayback: bool,
    #[serde(default)]
    disablepictureinpicture: bool,
    #[serde(default)]
    source_count: usize,
    #[serde(default)]
    sources: Vec<ExpectedMediaSource>,
    #[serde(default)]
    track_count: usize,
    #[serde(default)]
    default_track_count: usize,
    #[serde(default)]
    tracks: Vec<ExpectedMediaTrack>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMediaResourceDescriptor {
    media_index: usize,
    media_kind: String,
    element: String,
    resource_kind: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    track_kind: Option<String>,
    #[serde(default)]
    srclang: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    default_track: bool,
    candidate_kind: String,
    #[serde(default)]
    media_resource_blocked: bool,
    #[serde(default)]
    media_resource_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedEmbeddedContext {
    element: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    browsing_context_name: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    csp: Option<String>,
    #[serde(default)]
    sandbox: Vec<String>,
    #[serde(default)]
    allow: Option<String>,
    #[serde(default)]
    allowfullscreen: bool,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    srcdoc: Option<String>,
    #[serde(default)]
    credentialless: bool,
    #[serde(default)]
    fallback_text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedEmbeddedPolicyDescriptor {
    element: String,
    #[serde(default)]
    resource_kind: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    resolved_url: Option<String>,
    #[serde(default)]
    browsing_context_name: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    loading: Option<String>,
    #[serde(default)]
    fetchpriority: Option<String>,
    #[serde(default)]
    csp: Option<String>,
    #[serde(default)]
    sandbox: Vec<String>,
    #[serde(default)]
    sandbox_token_count: usize,
    #[serde(default)]
    allow: Option<String>,
    #[serde(default)]
    allow_tokens: Vec<String>,
    #[serde(default)]
    allow_token_count: usize,
    #[serde(default)]
    allowfullscreen: bool,
    #[serde(default)]
    fullscreen_allowed: bool,
    #[serde(default)]
    referrerpolicy: Option<String>,
    #[serde(default)]
    srcdoc: Option<String>,
    #[serde(default)]
    has_srcdoc: bool,
    #[serde(default)]
    credentialless: bool,
    #[serde(default)]
    fallback_text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedInteractiveElement {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_owns: Vec<String>,
    #[serde(default)]
    aria_activedescendant: Option<String>,
    #[serde(default)]
    aria_current: Option<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_modal: Option<String>,
    #[serde(default)]
    aria_pressed: Option<String>,
    #[serde(default)]
    aria_selected: Option<String>,
    #[serde(default)]
    aria_invalid: Option<String>,
    #[serde(default)]
    aria_live: Option<String>,
    #[serde(default)]
    aria_busy: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    aria_required: Option<String>,
    #[serde(default)]
    aria_keyshortcuts: Vec<String>,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    open: bool,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    focusable: Option<bool>,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    draggable: Option<String>,
    #[serde(default)]
    draggable_state: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    translate: Option<String>,
    #[serde(default)]
    popover: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedFocusNavigationDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    focus_kind: String,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    sequential_focus: bool,
    #[serde(default)]
    programmatic_focus: bool,
    #[serde(default)]
    focus_blocked: bool,
    #[serde(default)]
    focus_block_reasons: Vec<String>,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    tabindex_order: Option<i32>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_activedescendant: Option<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedKeyboardInteractionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    keyboard_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    sequential_focus: bool,
    #[serde(default)]
    programmatic_focus: bool,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    tabindex_order: Option<i32>,
    #[serde(default)]
    accesskey: Vec<String>,
    #[serde(default)]
    aria_keyshortcuts: Vec<String>,
    #[serde(default)]
    keyboard_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    aria_controls: Vec<String>,
    #[serde(default)]
    aria_activedescendant: Option<String>,
    #[serde(default)]
    aria_expanded: Option<String>,
    #[serde(default)]
    aria_haspopup: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    keyboard_blocked: bool,
    #[serde(default)]
    keyboard_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedInputPlanningDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    input_kind: String,
    #[serde(default)]
    control_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    placeholder: Option<String>,
    value: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    #[serde(default)]
    autocapitalize: Option<String>,
    #[serde(default)]
    enterkeyhint: Option<String>,
    #[serde(default)]
    dirname: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    autocorrect: Option<String>,
    #[serde(default)]
    inputmode: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    min: Option<String>,
    #[serde(default)]
    max: Option<String>,
    #[serde(default)]
    step: Option<String>,
    #[serde(default)]
    minlength: Option<String>,
    #[serde(default)]
    maxlength: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    rows: Option<String>,
    #[serde(default)]
    cols: Option<String>,
    #[serde(default)]
    wrap: Option<String>,
    #[serde(default)]
    list: Option<String>,
    #[serde(default)]
    datalist_options: Vec<String>,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    input_handlers: Vec<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    input_blocked: bool,
    #[serde(default)]
    input_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDragDropDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    drag_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    draggable: Option<String>,
    #[serde(default)]
    draggable_state: Option<String>,
    #[serde(default)]
    drag_source: bool,
    #[serde(default)]
    drop_target: bool,
    #[serde(default)]
    drag_handlers: Vec<String>,
    #[serde(default)]
    drop_handlers: Vec<String>,
    #[serde(default)]
    pointer_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    drag_blocked: bool,
    #[serde(default)]
    drag_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedClipboardInteractionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    clipboard_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    clipboard_handlers: Vec<String>,
    #[serde(default)]
    copy_handlers: Vec<String>,
    #[serde(default)]
    cut_handlers: Vec<String>,
    #[serde(default)]
    paste_handlers: Vec<String>,
    #[serde(default)]
    input_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    clipboard_blocked: bool,
    #[serde(default)]
    clipboard_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedSelectionInteractionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    selection_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    selection_handlers: Vec<String>,
    #[serde(default)]
    select_handlers: Vec<String>,
    #[serde(default)]
    selection_change_handlers: Vec<String>,
    #[serde(default)]
    input_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    selection_blocked: bool,
    #[serde(default)]
    selection_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCompositionInteractionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    #[serde(default)]
    source: String,
    composition_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    inputmode: Option<String>,
    #[serde(default)]
    enterkeyhint: Option<String>,
    #[serde(default)]
    composition_handlers: Vec<String>,
    #[serde(default)]
    composition_start_handlers: Vec<String>,
    #[serde(default)]
    composition_update_handlers: Vec<String>,
    #[serde(default)]
    composition_end_handlers: Vec<String>,
    #[serde(default)]
    beforeinput_handlers: Vec<String>,
    #[serde(default)]
    input_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    composition_blocked: bool,
    #[serde(default)]
    composition_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedPointerInteractionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    pointer_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_for: Option<String>,
    #[serde(default)]
    popover_target: Option<String>,
    #[serde(default)]
    popover_target_action: Option<String>,
    #[serde(default)]
    contenteditable: Option<String>,
    #[serde(default)]
    editing_mode: Option<String>,
    #[serde(default)]
    draggable: Option<String>,
    #[serde(default)]
    draggable_state: Option<String>,
    #[serde(default)]
    pointer_handlers: Vec<String>,
    #[serde(default)]
    mouse_handlers: Vec<String>,
    #[serde(default)]
    touch_handlers: Vec<String>,
    #[serde(default)]
    wheel_handlers: Vec<String>,
    #[serde(default)]
    click_handlers: Vec<String>,
    #[serde(default)]
    drag_handlers: Vec<String>,
    #[serde(default)]
    drop_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    pointer_blocked: bool,
    #[serde(default)]
    pointer_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedScrollInteractionDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    authored_role: Option<String>,
    #[serde(default)]
    source: String,
    scroll_kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_valuenow: Option<String>,
    #[serde(default)]
    aria_valuemin: Option<String>,
    #[serde(default)]
    aria_valuemax: Option<String>,
    #[serde(default)]
    aria_valuetext: Option<String>,
    #[serde(default)]
    aria_orientation: Option<String>,
    #[serde(default)]
    aria_disabled: Option<String>,
    #[serde(default)]
    aria_readonly: Option<String>,
    #[serde(default)]
    tabindex: Option<String>,
    #[serde(default)]
    scroll_handlers: Vec<String>,
    #[serde(default)]
    wheel_handlers: Vec<String>,
    #[serde(default)]
    touch_handlers: Vec<String>,
    #[serde(default)]
    handler_count: usize,
    #[serde(default)]
    focusable: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    inert: bool,
    #[serde(default)]
    aria_hidden: bool,
    #[serde(default)]
    scroll_blocked: bool,
    #[serde(default)]
    scroll_block_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDisclosure {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    summary_text: Option<String>,
    #[serde(default)]
    open: bool,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    #[serde(default)]
    aria_modal: Option<String>,
    #[serde(default)]
    closedby: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDisclosureStateDescriptor {
    element: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    disclosure_kind: String,
    #[serde(default)]
    open: bool,
    #[serde(default)]
    grouped: bool,
    #[serde(default)]
    group_name: Option<String>,
    #[serde(default)]
    has_summary: bool,
    #[serde(default)]
    summary_text: Option<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    aria_label: Option<String>,
    #[serde(default)]
    aria_labelledby: Vec<String>,
    #[serde(default)]
    aria_describedby: Vec<String>,
    #[serde(default)]
    aria_modal: Option<String>,
    #[serde(default)]
    modal: bool,
    #[serde(default)]
    closedby: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    text_length: usize,
}

#[derive(Debug, Deserialize)]
struct ExpectedForm {
    #[serde(default)]
    id: Option<String>,
    action: Option<String>,
    resolved_action: Option<String>,
    #[serde(default)]
    name: Option<String>,
    method: String,
    enctype: Option<String>,
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    accept_charset: Option<String>,
    #[serde(default)]
    accept_charset_tokens: Vec<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    #[serde(default)]
    rel: Option<String>,
    #[serde(default)]
    rel_tokens: Vec<String>,
    #[serde(default)]
    rel_external: bool,
    #[serde(default)]
    rel_nofollow: bool,
    #[serde(default)]
    rel_noopener: bool,
    #[serde(default)]
    rel_noreferrer: bool,
    #[serde(default)]
    novalidate: bool,
    #[serde(default)]
    event_handlers: Vec<String>,
    #[serde(default)]
    fieldsets: Vec<ExpectedFormFieldset>,
    #[serde(default)]
    labels: Vec<ExpectedFormLabel>,
    #[serde(default)]
    datalists: Vec<ExpectedFormDatalist>,
    #[serde(default)]
    selects: Vec<ExpectedFormSelect>,
    #[serde(default)]
    outputs: Vec<ExpectedFormOutput>,
    #[serde(default)]
    measurements: Vec<ExpectedFormMeasurement>,
    #[serde(default)]
    object_controls: Vec<ExpectedFormObject>,
    #[serde(default)]
    successful_controls: Vec<ExpectedFormSuccessfulControl>,
    #[serde(default)]
    validation_controls: Vec<ExpectedFormValidationControl>,
    #[serde(default)]
    buttons: Vec<ExpectedFormButton>,
    #[serde(default)]
    text_entries: Vec<ExpectedFormTextEntry>,
    #[serde(default)]
    choice_controls: Vec<ExpectedFormChoiceControl>,
    #[serde(default)]
    file_controls: Vec<ExpectedFormFileControl>,
    #[serde(default)]
    hidden_controls: Vec<ExpectedFormHiddenControl>,
    #[serde(default)]
    image_controls: Vec<ExpectedFormImageControl>,
    controls: Vec<ExpectedFormControl>,
    #[serde(default)]
    submitters: Vec<ExpectedFormSubmitter>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormFieldset {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    legend: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    control_ids: Vec<String>,
    #[serde(default)]
    control_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormLabel {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    for_control: Option<String>,
    text: String,
    #[serde(default)]
    control_id: Option<String>,
    #[serde(default)]
    control_name: Option<String>,
    #[serde(default)]
    control_type: Option<String>,
    association: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormDatalist {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    control_ids: Vec<String>,
    #[serde(default)]
    control_names: Vec<String>,
    #[serde(default)]
    options: Vec<ExpectedDatalistOption>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDatalistOption {
    value: String,
    #[serde(default)]
    label: Option<String>,
    text: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormSelect {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    selected_options: Vec<String>,
    #[serde(default)]
    options: Vec<ExpectedSelectOption>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormOutput {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    for_tokens: Vec<String>,
    #[serde(default)]
    for_control_ids: Vec<String>,
    #[serde(default)]
    for_control_names: Vec<String>,
    #[serde(default)]
    for_control_types: Vec<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_barred_reason: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormMeasurement {
    #[serde(default)]
    id: Option<String>,
    measurement_type: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    min: Option<String>,
    #[serde(default)]
    max: Option<String>,
    #[serde(default)]
    low: Option<String>,
    #[serde(default)]
    high: Option<String>,
    #[serde(default)]
    optimum: Option<String>,
    #[serde(default)]
    indeterminate: bool,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormObject {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    data: Option<String>,
    #[serde(default)]
    resolved_data: Option<String>,
    #[serde(default)]
    type_hint: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    usemap: Option<String>,
    #[serde(default)]
    fallback_text: String,
    #[serde(default)]
    params: Vec<ExpectedFormObjectParam>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormObjectParam {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormSuccessfulControl {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    name: String,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    submission_values: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormValidationControl {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormTextEntry {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    placeholder: Option<String>,
    value: Option<String>,
    text: String,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    #[serde(default)]
    autocapitalize: Option<String>,
    #[serde(default)]
    enterkeyhint: Option<String>,
    #[serde(default)]
    dirname: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    autocorrect: Option<String>,
    #[serde(default)]
    inputmode: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    min: Option<String>,
    #[serde(default)]
    max: Option<String>,
    #[serde(default)]
    step: Option<String>,
    #[serde(default)]
    minlength: Option<String>,
    #[serde(default)]
    maxlength: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    rows: Option<String>,
    #[serde(default)]
    cols: Option<String>,
    #[serde(default)]
    wrap: Option<String>,
    #[serde(default)]
    list: Option<String>,
    #[serde(default)]
    datalist_options: Vec<String>,
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormChoiceControl {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    value: Option<String>,
    checked: bool,
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    group_required: bool,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
    #[serde(default)]
    group_name: Option<String>,
    #[serde(default)]
    group_checked_ids: Vec<String>,
    #[serde(default)]
    group_checked_values: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormFileControl {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accept: Option<String>,
    #[serde(default)]
    accept_tokens: Vec<String>,
    #[serde(default)]
    capture: Option<String>,
    #[serde(default)]
    multiple: bool,
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormHiddenControl {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    disabled: bool,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_barred_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormImageControl {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    disabled: bool,
    #[serde(default)]
    autofocus: bool,
    #[serde(default)]
    submitter: bool,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    resolved_action: Option<String>,
    method: String,
    #[serde(default)]
    enctype: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    novalidate: bool,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    coordinate_names: Vec<String>,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_barred_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormControl {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    accessible_description: Option<String>,
    #[serde(default)]
    placeholder: Option<String>,
    #[serde(default)]
    autocomplete: Option<String>,
    #[serde(default)]
    autocomplete_tokens: Vec<String>,
    #[serde(default)]
    autocapitalize: Option<String>,
    #[serde(default)]
    enterkeyhint: Option<String>,
    #[serde(default)]
    dirname: Option<String>,
    #[serde(default)]
    spellcheck: Option<String>,
    #[serde(default)]
    autocorrect: Option<String>,
    #[serde(default)]
    accept: Option<String>,
    #[serde(default)]
    accept_tokens: Vec<String>,
    #[serde(default)]
    capture: Option<String>,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
    #[serde(default)]
    inputmode: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    min: Option<String>,
    #[serde(default)]
    max: Option<String>,
    #[serde(default)]
    step: Option<String>,
    #[serde(default)]
    minlength: Option<String>,
    #[serde(default)]
    maxlength: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    rows: Option<String>,
    #[serde(default)]
    cols: Option<String>,
    #[serde(default)]
    wrap: Option<String>,
    #[serde(default)]
    list: Option<String>,
    #[serde(default)]
    datalist_options: Vec<String>,
    #[serde(default)]
    output_for: Vec<String>,
    #[serde(default)]
    form_action: Option<String>,
    #[serde(default)]
    resolved_form_action: Option<String>,
    #[serde(default)]
    form_enctype: Option<String>,
    #[serde(default)]
    form_method: Option<String>,
    #[serde(default)]
    form_target: Option<String>,
    #[serde(default)]
    form_novalidate: bool,
    value: Option<String>,
    #[serde(default)]
    successful: bool,
    #[serde(default)]
    submission_values: Vec<String>,
    #[serde(default)]
    autofocus: bool,
    disabled: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    will_validate: bool,
    #[serde(default)]
    validation_attributes: Vec<String>,
    #[serde(default)]
    validation_barred_reason: Option<String>,
    checked: bool,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    selected_options: Vec<String>,
    #[serde(default)]
    option_items: Vec<ExpectedSelectOption>,
    text: String,
    options: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedSelectOption {
    value: String,
    #[serde(default)]
    label: Option<String>,
    text: String,
    #[serde(default)]
    selected: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    group_label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormSubmitter {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    resolved_action: Option<String>,
    method: String,
    #[serde(default)]
    enctype: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    novalidate: bool,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormButton {
    #[serde(default)]
    id: Option<String>,
    control_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    form_owner: Option<String>,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    autofocus: bool,
    #[serde(default)]
    submitter: bool,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    resolved_action: Option<String>,
    method: String,
    #[serde(default)]
    enctype: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    effective_target: Option<String>,
    #[serde(default)]
    novalidate: bool,
    #[serde(default)]
    value: Option<String>,
    text: String,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    resolved_src: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    width: Option<String>,
    #[serde(default)]
    height: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTable {
    caption: Option<String>,
    row_count: usize,
    #[serde(default)]
    column_count: usize,
    #[serde(default)]
    column_hint_count: usize,
    cell_count: usize,
    header_cell_count: usize,
}

#[derive(Debug, Deserialize)]
struct ExpectedTableCell {
    table_index: usize,
    #[serde(default)]
    table_id: Option<String>,
    #[serde(default)]
    table_caption: Option<String>,
    #[serde(default)]
    section_kind: Option<String>,
    row_index: usize,
    column_index: usize,
    element: String,
    #[serde(default)]
    id: Option<String>,
    text: String,
    #[serde(default)]
    accessible_name: Option<String>,
    #[serde(default)]
    header: bool,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    headers: Vec<String>,
    #[serde(default)]
    abbr: Option<String>,
    #[serde(default)]
    rowspan: Option<String>,
    #[serde(default)]
    colspan: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTableStructureDescriptor {
    table_index: usize,
    #[serde(default)]
    table_id: Option<String>,
    #[serde(default)]
    caption: Option<String>,
    row_count: usize,
    #[serde(default)]
    column_count: usize,
    #[serde(default)]
    column_hint_count: usize,
    cell_count: usize,
    header_cell_count: usize,
    #[serde(default)]
    section_kinds: Vec<String>,
    #[serde(default)]
    header_scopes: Vec<String>,
    #[serde(default)]
    header_ids: Vec<String>,
    #[serde(default)]
    cells_with_headers_count: usize,
    #[serde(default)]
    spanning_cell_count: usize,
    #[serde(default)]
    table_blocked: bool,
    #[serde(default)]
    table_block_reasons: Vec<String>,
}

#[test]
fn browser_readiness_cases_extract_browser_document_facts() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");

    assert_eq!(suite.format, "venture-html-browser-readiness/v1");
    assert_eq!(suite.suite, "browser-readiness");
    assert!(!suite.cases.is_empty(), "fixture should contain cases");

    for case in suite.cases {
        let actual = parse_browser_document(&case.input)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", case.id));
        let tracks_aria_name_descriptors = case.expected.aria_name_descriptors.is_some();
        let tracks_aria_description_descriptors =
            case.expected.aria_description_descriptors.is_some();
        let tracks_template_descriptors = case.expected.template_descriptors.is_some();
        let tracks_slot_descriptors = case.expected.slot_descriptors.is_some();
        let tracks_custom_element_descriptors = case.expected.custom_element_descriptors.is_some();
        let tracks_canvas_descriptors = case.expected.canvas_descriptors.is_some();
        let tracks_component_hydration_descriptors =
            case.expected.component_hydration_descriptors.is_some();
        let tracks_structured_data_descriptors =
            case.expected.structured_data_descriptors.is_some();
        let tracks_table_structure_descriptors =
            case.expected.table_structure_descriptors.is_some();
        let tracks_image_map_descriptors = case.expected.image_map_descriptors.is_some();
        let tracks_link_resource_descriptors = case.expected.link_resource_descriptors.is_some();
        let tracks_form_control_descriptors = case.expected.form_control_descriptors.is_some();
        let tracks_media_resource_descriptors = case.expected.media_resource_descriptors.is_some();
        let tracks_anchor_descriptors = case.expected.anchor_descriptors.is_some();
        let tracks_heading_descriptors = case.expected.heading_descriptors.is_some();
        let tracks_text_semantic_descriptors = case.expected.text_semantic_descriptors.is_some();
        let tracks_text_flow_descriptors = case.expected.text_flow_descriptors.is_some();
        let tracks_popover_descriptors = case.expected.popover_descriptors.is_some();
        let tracks_aria_collection_descriptors =
            case.expected.aria_collection_descriptors.is_some();
        let mut expected = case.expected.into_browser_document();
        if !tracks_aria_name_descriptors {
            expected.aria_name_descriptors = actual.aria_name_descriptors.clone();
        }
        if !tracks_aria_description_descriptors {
            expected.aria_description_descriptors = actual.aria_description_descriptors.clone();
        }
        if !tracks_template_descriptors {
            expected.template_descriptors = actual.template_descriptors.clone();
        }
        if !tracks_slot_descriptors {
            expected.slot_descriptors = actual.slot_descriptors.clone();
        }
        if !tracks_custom_element_descriptors {
            expected.custom_element_descriptors = actual.custom_element_descriptors.clone();
        }
        if !tracks_canvas_descriptors {
            expected.canvas_descriptors = actual.canvas_descriptors.clone();
        }
        if !tracks_component_hydration_descriptors {
            expected.component_hydration_descriptors =
                actual.component_hydration_descriptors.clone();
        }
        if !tracks_structured_data_descriptors {
            expected.structured_data_descriptors = actual.structured_data_descriptors.clone();
        }
        if !tracks_table_structure_descriptors {
            expected.table_structure_descriptors = actual.table_structure_descriptors.clone();
        }
        if !tracks_image_map_descriptors {
            expected.image_map_descriptors = actual.image_map_descriptors.clone();
        }
        if !tracks_link_resource_descriptors {
            expected.link_resource_descriptors = actual.link_resource_descriptors.clone();
        }
        if !tracks_form_control_descriptors {
            expected.form_control_descriptors = actual.form_control_descriptors.clone();
        }
        if !tracks_media_resource_descriptors {
            expected.media_resource_descriptors = actual.media_resource_descriptors.clone();
        }
        if !tracks_anchor_descriptors {
            expected.anchor_descriptors = actual.anchor_descriptors.clone();
        }
        if !tracks_heading_descriptors {
            expected.heading_descriptors = actual.heading_descriptors.clone();
        }
        if !tracks_text_semantic_descriptors {
            expected.text_semantic_descriptors = actual.text_semantic_descriptors.clone();
        }
        if !tracks_text_flow_descriptors {
            expected.text_flow_descriptors = actual.text_flow_descriptors.clone();
        }
        if !tracks_popover_descriptors {
            expected.popover_descriptors = actual.popover_descriptors.clone();
        }
        if !tracks_aria_collection_descriptors {
            expected.aria_collection_descriptors = actual.aria_collection_descriptors.clone();
        }

        assert_eq!(
            actual, expected,
            "{} extracted browser facts should match",
            case.id
        );
    }
}

#[test]
fn browser_readiness_completion_manifest_matches_public_surface() {
    let fixture: serde_json::Value = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let cases = fixture["cases"]
        .as_array()
        .expect("browser readiness fixture should contain cases");

    for expectation in BROWSER_READINESS_COMPLETION_SURFACE {
        let document_field = format!("pub {}: Vec<{}>,", expectation.field, expectation.type_name);
        assert!(
            HTML_PARSER_SOURCE.contains(&document_field),
            "BrowserDocument should expose `{}` as `{}`",
            expectation.field,
            expectation.type_name,
        );

        if let Some(case_id) = expectation.fixture_case {
            let case = cases
                .iter()
                .find(|case| case["id"].as_str() == Some(case_id))
                .unwrap_or_else(|| panic!("completion fixture case `{case_id}` should exist"));
            let expected = case["expected"].as_object().unwrap_or_else(|| {
                panic!("completion fixture case `{case_id}` should have expected facts")
            });
            let values = expected
                .get(expectation.field)
                .unwrap_or_else(|| {
                    panic!(
                        "completion fixture case `{case_id}` should pin `{}`",
                        expectation.field
                    )
                })
                .as_array()
                .unwrap_or_else(|| {
                    panic!(
                        "completion fixture field `{}` should be an array",
                        expectation.field
                    )
                });
            assert!(
                !values.is_empty(),
                "completion fixture case `{case_id}` should pin non-empty `{}`",
                expectation.field,
            );
        }

        for test_name in expectation.focused_tests {
            let test_signature = format!("fn {test_name}(");
            assert!(
                BROWSER_READINESS_TEST_SOURCE.contains(&test_signature),
                "completion surface `{}` should be covered by focused test `{}`",
                expectation.field,
                test_name,
            );
        }

        assert!(
            expectation.fixture_case.is_some() || !expectation.focused_tests.is_empty(),
            "completion surface `{}` should have fixture or focused-test evidence",
            expectation.field,
        );
    }
}

#[test]
fn browser_table_cell_descriptor_metadata_tracks_headers_spans_and_sections() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "table-cell-descriptor-page")
        .expect("table cell descriptor fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("table cell descriptor fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.table_cells, expected.table_cells,
        "table cell descriptors should preserve table context, sections, spans, headers, and grid positions",
    );
}

#[test]
fn browser_table_structure_descriptors_track_sections_headers_spans_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "table-cell-descriptor-page")
        .expect("table cell descriptor fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("table structure descriptor fixture should parse into browser document facts");

    assert_eq!(
        actual.table_structure_descriptors,
        case.expected
            .into_browser_document()
            .table_structure_descriptors,
        "table structure descriptors should preserve sections, header scopes, ids, spans, header references, and blocker metadata",
    );
}

#[test]
fn browser_table_structure_descriptors_track_layout_blockers() {
    let actual =
        parse_browser_document("<body><table><colgroup><col span=3><tr><td>Loose<td>Cells</table>")
            .expect("blocked table fixture should parse into browser document facts");

    assert_eq!(actual.table_structure_descriptors.len(), 1);
    let descriptor = &actual.table_structure_descriptors[0];

    assert!(descriptor.table_blocked);
    assert_eq!(
        descriptor.table_block_reasons,
        vec![
            "missing-caption",
            "missing-header-cells",
            "column-hint-count-mismatch"
        ],
    );
}

#[test]
fn browser_script_style_security_metadata_tracks_nonces_and_fetch_policy() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "script-style-loading-page")
        .expect("script/style loading fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("script/style loading fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.metadata.resource_hints, expected.metadata.resource_hints,
        "resource hints should preserve CSP nonces and fetch policy metadata",
    );
    assert_eq!(
        actual.resources, expected.resources,
        "resources should preserve script and stylesheet security metadata",
    );
    assert_eq!(
        actual.scripts, expected.scripts,
        "scripts should preserve CSP nonces and loading policy metadata",
    );
    assert_eq!(
        actual.stylesheets, expected.stylesheets,
        "stylesheets should preserve CSP nonces and loading policy metadata",
    );
}

#[test]
fn browser_script_storage_access_descriptors_track_flat_storage_api_hints() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "script-storage-access-page")
        .expect("script storage access fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("script storage access fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.scripts, expected.scripts,
        "scripts should preserve storage-relevant inline text and module state",
    );
    assert_eq!(
        actual.script_storage_access_descriptors, expected.script_storage_access_descriptors,
        "script storage access descriptors should preserve storage API targets and blockers",
    );
}

#[test]
fn browser_script_worker_messaging_descriptors_track_flat_worker_and_channel_hints() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "script-worker-messaging-page")
        .expect("script worker messaging fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("script worker messaging fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.scripts, expected.scripts,
        "scripts should preserve worker-relevant inline text and module state",
    );
    assert_eq!(
        actual.script_worker_messaging_descriptors, expected.script_worker_messaging_descriptors,
        "script worker messaging descriptors should preserve worker targets, channels, and blockers",
    );
}

#[test]
fn browser_script_module_graph_descriptors_track_flat_import_maps_and_preloads() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "script-module-graph-page")
        .expect("script module graph fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("script module graph fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.scripts, expected.scripts,
        "scripts should preserve import maps, module text, and fallback import hints",
    );
    assert_eq!(
        actual.script_module_graph_descriptors, expected.script_module_graph_descriptors,
        "script module graph descriptors should preserve imports, preloads, and blockers",
    );
}

#[test]
fn browser_document_policy_descriptors_track_permissions_policy_features() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "document-metadata-policy-page")
        .expect("document metadata policy fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("document metadata policy fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.document_policy_descriptors, expected.document_policy_descriptors,
        "document policy descriptors should preserve raw and normalized permissions policy metadata",
    );
    assert_eq!(
        actual.document_policy_descriptors[0].permissions_policy_features,
        vec!["geolocation".to_string(), "camera".to_string()],
        "document permissions policy features should be normalized for planner consumption",
    );
    assert_eq!(
        actual.document_policy_descriptors[0].permissions_policy_feature_count, 2,
        "document permissions policy feature count should match normalized features",
    );
}

#[test]
fn browser_embedded_policy_descriptors_track_allow_policy_features() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "embedded-resource-page")
        .expect("embedded resource fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("embedded resource fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.embedded_policy_descriptors, expected.embedded_policy_descriptors,
        "embedded policy descriptors should preserve raw and normalized allow policy metadata",
    );

    let frame = &actual.embedded_policy_descriptors[0];
    assert_eq!(
        frame.allow_tokens,
        vec!["fullscreen".to_string(), "geolocation".to_string()],
        "iframe allow features should be normalized from semicolon-delimited directives",
    );
    assert_eq!(frame.allow_token_count, 2);
    assert!(frame.fullscreen_allowed);

    let inline = &actual.embedded_policy_descriptors[3];
    assert_eq!(inline.allow_tokens, vec!["payment".to_string()]);
    assert_eq!(inline.allow_token_count, 1);
    assert!(!inline.fullscreen_allowed);
}

#[test]
fn browser_resource_priority_metadata_tracks_rel_and_blocking_tokens() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "script-style-loading-page")
        .expect("script style loading fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("script style loading fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.metadata.resource_hints, expected.metadata.resource_hints,
        "resource hints should preserve rel and blocking token metadata",
    );
    assert_eq!(
        actual.resources, expected.resources,
        "resources should preserve rel and blocking token metadata",
    );
    assert_eq!(
        actual.scripts, expected.scripts,
        "scripts should preserve blocking token metadata",
    );
    assert_eq!(
        actual.stylesheets, expected.stylesheets,
        "stylesheets should preserve rel and blocking token metadata",
    );
}

#[test]
fn browser_link_descriptor_metadata_tracks_icon_and_alternate_fields() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "link-resource-metadata-page")
        .expect("link resource fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("link resource fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.resources, expected.resources,
        "link resources should preserve icon sizes, mask colors, and alternate language descriptors",
    );
}

#[test]
fn browser_link_resource_descriptors_track_rel_hints_policy_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "link-resource-metadata-page")
        .expect("link resource fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("link resource fixture should parse into browser document facts");

    assert_eq!(
        actual.link_resource_descriptors,
        case.expected.into_browser_document().link_resource_descriptors,
        "link resource descriptors should preserve relation kinds, scheduling hints, icon/alternate metadata, and blocker state",
    );
}

#[test]
fn browser_link_resource_descriptors_track_missing_and_unresolved_hints() {
    let actual = parse_browser_document(
        r#"<link rel=preload href=font.woff2>
           <link rel=preload href=hero.jpg as=image imagesrcset="hero.jpg 1x">
           <link rel=icon href=favicon.ico>
           <link rel=mask-icon href=mask.svg>
           <link rel=alternate href=feed.xml>"#,
    )
    .expect("blocked link resource descriptor fixture should parse");

    assert_eq!(actual.link_resource_descriptors.len(), 5);
    assert_eq!(
        actual.link_resource_descriptors[0].resource_block_reasons,
        vec!["unresolved-url", "preload-missing-as"],
    );
    assert_eq!(
        actual.link_resource_descriptors[1].resource_block_reasons,
        vec!["unresolved-url", "responsive-image-preload-missing-sizes",],
    );
    assert_eq!(
        actual.link_resource_descriptors[2].resource_block_reasons,
        vec!["unresolved-url", "icon-missing-size-or-type"],
    );
    assert_eq!(
        actual.link_resource_descriptors[3].resource_block_reasons,
        vec!["unresolved-url", "mask-icon-missing-color"],
    );
    assert_eq!(
        actual.link_resource_descriptors[4].resource_block_reasons,
        vec!["unresolved-url", "alternate-missing-descriptor"],
    );
}

#[test]
fn browser_image_map_descriptor_metadata_tracks_area_navigation() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "responsive-image-metadata-page")
        .expect("responsive image fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("responsive image fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.image_maps, expected.image_maps,
        "image maps should preserve area geometry, link policy, and resolved navigation metadata",
    );
}

#[test]
fn browser_image_map_descriptors_track_map_links_area_coverage_and_navigation() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "responsive-image-metadata-page")
        .expect("responsive image fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("responsive image fixture should parse into browser document facts");

    assert_eq!(
        actual.image_map_descriptors,
        case.expected.into_browser_document().image_map_descriptors,
        "image map descriptors should preserve image usemap links, area geometry coverage, navigation counts, and blocker state",
    );
}

#[test]
fn browser_image_map_descriptors_track_unresolved_and_blocked_maps() {
    let actual = parse_browser_document(
        r##"<body>
            <img src="hero.png" usemap="#missing" alt="Hero">
            <map id="empty"></map>
            <map name="empty-shapes">
              <area alt="No href">
              <area href="details.html" coords="0,0,20,20">
            </map>
        </body>"##,
    )
    .expect("blocked image map descriptor fixture should parse");

    assert_eq!(actual.image_map_descriptors.len(), 3);

    let missing = actual
        .image_map_descriptors
        .iter()
        .find(|descriptor| descriptor.name.as_deref() == Some("missing"))
        .expect("missing usemap descriptor should be present");
    assert!(missing.map_blocked);
    assert_eq!(missing.referenced_image_sources, vec!["hero.png"]);
    assert_eq!(missing.map_block_reasons, vec!["missing-map"]);

    let unnamed = actual
        .image_map_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("empty"))
        .expect("empty map descriptor should be present");
    assert!(unnamed.map_blocked);
    assert_eq!(
        unnamed.map_block_reasons,
        vec!["missing-name", "missing-areas", "unreferenced-map"],
    );

    let blocked = actual
        .image_map_descriptors
        .iter()
        .find(|descriptor| descriptor.name.as_deref() == Some("empty-shapes"))
        .expect("blocked area map descriptor should be present");
    assert_eq!(blocked.area_count, 2);
    assert_eq!(blocked.navigable_area_count, 1);
    assert_eq!(blocked.missing_alt_area_count, 1);
    assert_eq!(blocked.missing_href_area_count, 1);
    assert_eq!(blocked.missing_coords_area_count, 1);
    assert_eq!(
        blocked.map_block_reasons,
        vec![
            "unreferenced-map",
            "areas-without-href",
            "areas-without-alt",
            "areas-without-coords",
        ],
    );
}

#[test]
fn browser_structured_data_descriptors_track_item_identity_refs_and_properties() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "structured-data-microdata-page")
        .expect("structured data fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("structured data fixture should parse into browser document facts");

    assert_eq!(
        actual.structured_data_descriptors,
        case.expected
            .into_browser_document()
            .structured_data_descriptors,
        "structured data descriptors should preserve item identity, itemref resolution, property names, and URL property counts",
    );
}

#[test]
fn browser_structured_data_descriptors_track_missing_and_unresolved_blockers() {
    let actual = parse_browser_document(
        r#"<body>
            <article id=empty itemscope itemref="missing-one missing-two"></article>
            <section id=typed itemscope itemtype="https://schema.org/Thing" itemref=extra>
                <span itemprop=name>Named item</span>
            </section>
            <p id=extra itemprop=description>Extra description</p>
        </body>"#,
    )
    .expect("structured data blocker fixture should parse");

    let empty = actual
        .structured_data_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("empty"))
        .expect("empty structured data descriptor should be present");

    assert!(empty.item_type.is_empty());
    assert_eq!(empty.item_ref, vec!["missing-one", "missing-two"]);
    assert_eq!(empty.item_ref_count, 2);
    assert_eq!(
        empty.unresolved_item_refs,
        vec!["missing-one", "missing-two"]
    );
    assert_eq!(empty.property_count, 0);
    assert!(empty.structured_data_blocked);
    assert_eq!(
        empty.structured_data_block_reasons,
        vec![
            "missing-itemtype",
            "missing-properties",
            "unresolved-itemref"
        ]
    );

    let typed = actual
        .structured_data_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("typed"))
        .expect("typed structured data descriptor should be present");

    assert_eq!(typed.item_type_count, 1);
    assert_eq!(typed.item_ref, vec!["extra"]);
    assert!(typed.unresolved_item_refs.is_empty());
    assert_eq!(typed.property_names, vec!["name", "description"]);
    assert_eq!(typed.property_count, 2);
    assert!(!typed.structured_data_blocked);
}

#[test]
fn browser_text_semantic_descriptor_metadata_tracks_inline_annotations() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "inline-semantic-metadata-page")
        .expect("inline semantic fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("inline semantic fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.text_semantics, expected.text_semantics,
        "text semantics should preserve machine-readable values, edits, quotes, phrase semantics, ruby annotations, and bidi metadata",
    );
}

#[test]
fn browser_text_semantic_descriptors_track_kinds_values_and_cites() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "inline-semantic-metadata-page")
        .expect("inline semantic fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("inline semantic fixture should parse into browser document facts");

    assert_eq!(
        actual.text_semantic_descriptors,
        case.expected.into_browser_document().text_semantic_descriptors,
        "text semantic descriptors should preserve annotation kinds, machine-readable values, citations, ruby, bidi, and phrase metadata",
    );
}

#[test]
fn browser_text_semantic_descriptors_track_missing_and_unresolved_annotations() {
    let actual = parse_browser_document(
        r#"<data id=version>Version</data>
           <time id=date>soon</time>
           <q id=quote cite=notes/ref.html>quoted</q>
           <ins id=add cite=changes.html>Added</ins>
           <bdo id=rtl>abc</bdo>"#,
    )
    .expect("blocked text semantic descriptor fixture should parse");

    assert_eq!(actual.text_semantic_descriptors.len(), 5);
    assert_eq!(
        actual.text_semantic_descriptors[0].semantic_block_reasons,
        vec!["missing-data-value"],
    );
    assert_eq!(
        actual.text_semantic_descriptors[1].semantic_block_reasons,
        vec!["missing-datetime"],
    );
    assert_eq!(
        actual.text_semantic_descriptors[2].semantic_block_reasons,
        vec!["unresolved-quote-cite"],
    );
    assert_eq!(
        actual.text_semantic_descriptors[3].semantic_block_reasons,
        vec!["unresolved-edit-cite", "edit-missing-datetime"],
    );
    assert_eq!(
        actual.text_semantic_descriptors[4].semantic_block_reasons,
        vec!["bidi-missing-dir"],
    );
}

#[test]
fn browser_text_flow_descriptors_track_lists_quotes_and_preformatted_blocks() {
    let actual = parse_browser_document(
        r#"<base href="https://example.test/docs/">
           <ol id=steps start=3 type=A reversed>
             <li value=7>Install</li>
             <li>Run</li>
           </ol>
           <dl id=terms>
             <dt>API</dt>
             <dd>Application interface</dd>
           </dl>
           <blockquote id=quote cite=notes/ref.html>Quoted <q cite=#inline>inline</q></blockquote>
           <pre id=sample>  code
  block</pre>"#,
    )
    .expect("text flow descriptor fixture should parse");

    assert_eq!(actual.text_flow_descriptors.len(), 9);

    let ordered = &actual.text_flow_descriptors[0];
    assert_eq!(ordered.flow_index, 1);
    assert_eq!(ordered.element, "ol");
    assert_eq!(ordered.id.as_deref(), Some("steps"));
    assert_eq!(ordered.flow_kind, "list");
    assert_eq!(ordered.list_kind.as_deref(), Some("ordered"));
    assert_eq!(ordered.list_start.as_deref(), Some("3"));
    assert_eq!(ordered.list_marker_type.as_deref(), Some("A"));
    assert!(ordered.list_reversed);
    assert_eq!(ordered.list_item_count, 2);
    assert!(!ordered.flow_blocked);

    let valued_item = &actual.text_flow_descriptors[1];
    assert_eq!(valued_item.element, "li");
    assert_eq!(valued_item.flow_kind, "list-item");
    assert_eq!(valued_item.list_item_value.as_deref(), Some("7"));

    let description_list = &actual.text_flow_descriptors[3];
    assert_eq!(description_list.element, "dl");
    assert_eq!(description_list.flow_kind, "description-list");
    assert_eq!(
        description_list.description_list_kind.as_deref(),
        Some("description")
    );
    assert_eq!(description_list.term_count, 1);
    assert_eq!(description_list.description_count, 1);

    let quote = &actual.text_flow_descriptors[6];
    assert_eq!(quote.element, "blockquote");
    assert_eq!(quote.flow_kind, "quote");
    assert_eq!(quote.quote_cite.as_deref(), Some("notes/ref.html"));
    assert_eq!(
        quote.resolved_quote_cite.as_deref(),
        Some("https://example.test/docs/notes/ref.html")
    );

    let inline_quote = &actual.text_flow_descriptors[7];
    assert_eq!(inline_quote.element, "q");
    assert_eq!(inline_quote.flow_kind, "quote");
    assert_eq!(
        inline_quote.resolved_quote_cite.as_deref(),
        Some("https://example.test/docs/#inline")
    );

    let preformatted = &actual.text_flow_descriptors[8];
    assert_eq!(preformatted.element, "pre");
    assert_eq!(preformatted.flow_kind, "preformatted");
    assert_eq!(preformatted.text_flow.as_deref(), Some("preformatted"));
    assert_eq!(preformatted.text, "code block");
}

#[test]
fn browser_text_flow_descriptors_track_empty_and_unresolved_blockers() {
    let actual = parse_browser_document(
        r#"<ul id=empty></ul>
           <li id=orphan></li>
           <dl id=missing><dt></dt></dl>
           <blockquote cite=notes/ref.html>Quote</blockquote>
           <pre></pre>"#,
    )
    .expect("blocked text flow descriptor fixture should parse");

    assert_eq!(actual.text_flow_descriptors.len(), 6);
    assert_eq!(
        actual.text_flow_descriptors[0].flow_block_reasons,
        vec!["empty-list"],
    );
    assert_eq!(
        actual.text_flow_descriptors[1].flow_block_reasons,
        vec!["empty-list-item"],
    );
    assert_eq!(
        actual.text_flow_descriptors[2].flow_block_reasons,
        vec!["missing-description-details"],
    );
    assert_eq!(
        actual.text_flow_descriptors[3].flow_block_reasons,
        vec!["empty-description-item"],
    );
    assert_eq!(
        actual.text_flow_descriptors[4].flow_block_reasons,
        vec!["unresolved-quote-cite"],
    );
    assert_eq!(
        actual.text_flow_descriptors[5].flow_block_reasons,
        vec!["empty-preformatted"],
    );
}

#[test]
fn browser_anchor_descriptors_track_fragment_targets_and_duplicates() {
    let actual = parse_browser_document(
        "<h1 id=top>Top</h1>\
         <a name=legacy></a>\
         <section id=dup>First</section>\
         <p id=dup>Second</p>\
         <a id=both name=both>Both</a>",
    )
    .expect("anchor descriptor fixture should parse");

    assert_eq!(actual.anchor_descriptors.len(), 5);
    assert_eq!(actual.anchor_descriptors[0].anchor_kind, "id");
    assert_eq!(actual.anchor_descriptors[0].fragment_targets, vec!["top"]);
    assert!(!actual.anchor_descriptors[0].anchor_blocked);

    assert_eq!(actual.anchor_descriptors[1].anchor_kind, "named-anchor");
    assert_eq!(
        actual.anchor_descriptors[1].anchor_block_reasons,
        vec!["empty-fragment-target-text"],
    );

    assert!(actual.anchor_descriptors[2].duplicate_target);
    assert_eq!(
        actual.anchor_descriptors[2].anchor_block_reasons,
        vec!["duplicate-fragment-target"],
    );
    assert!(actual.anchor_descriptors[3].duplicate_target);

    assert_eq!(actual.anchor_descriptors[4].anchor_kind, "id-and-name");
    assert_eq!(actual.anchor_descriptors[4].fragment_targets, vec!["both"]);
}

#[test]
fn browser_heading_descriptors_track_outline_levels_and_blockers() {
    let actual = parse_browser_document("<h2>Start</h2><h4>Deep</h4><h3>Back</h3><h3></h3>")
        .expect("heading descriptor fixture should parse");

    assert_eq!(actual.heading_descriptors.len(), 4);
    assert_eq!(
        actual.heading_descriptors[0].outline_kind,
        "initial-skipped-level",
    );
    assert_eq!(
        actual.heading_descriptors[0].heading_block_reasons,
        vec!["skipped-heading-level"],
    );

    assert_eq!(actual.heading_descriptors[1].previous_level, Some(2));
    assert_eq!(actual.heading_descriptors[1].outline_kind, "skipped-level");
    assert!(actual.heading_descriptors[1].skipped_level);

    assert_eq!(
        actual.heading_descriptors[2].outline_kind,
        "ancestor-section",
    );
    assert!(!actual.heading_descriptors[2].heading_blocked);

    assert_eq!(actual.heading_descriptors[3].outline_kind, "sibling");
    assert_eq!(
        actual.heading_descriptors[3].heading_block_reasons,
        vec!["empty-heading-text"],
    );
}

#[test]
fn browser_navigation_group_descriptor_metadata_tracks_lists_and_landmarks() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "navigation-menu-descriptor-page")
        .expect("navigation menu fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("navigation menu fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.navigation_groups, expected.navigation_groups,
        "navigation groups should preserve list/menu landmark names, item counts, and ordered list metadata",
    );
}

#[test]
fn browser_navigation_group_descriptors_track_kinds_counts_and_labels() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "navigation-menu-descriptor-page")
        .expect("navigation menu fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("navigation menu fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.navigation_group_descriptors, expected.navigation_group_descriptors,
        "navigation group descriptors should classify landmarks, lists, menus, and readiness state",
    );
    assert_eq!(
        actual.navigation_group_descriptors[0].navigation_block_reasons,
        Vec::<String>::new(),
    );
    assert_eq!(
        actual.navigation_group_descriptors[3].group_kind, "list",
        "ordered lists should remain list descriptors with marker metadata",
    );
}

#[test]
fn browser_navigation_group_descriptors_track_missing_landmark_labels() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "document-outline-landmark-page")
        .expect("document outline fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("document outline fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.navigation_group_descriptors, expected.navigation_group_descriptors,
        "navigation group descriptors should report unlabeled navigation landmarks as blocked",
    );
    assert_eq!(
        actual.navigation_group_descriptors[0].navigation_block_reasons,
        vec!["missing-navigation-label"],
    );
}

#[test]
fn browser_section_landmark_descriptor_metadata_tracks_outline_roles_and_names() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "document-outline-landmark-page")
        .expect("document outline fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("document outline fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.section_landmarks, expected.section_landmarks,
        "section landmarks should preserve roles, accessible names, landmark kinds, and first headings",
    );
}

#[test]
fn browser_section_landmark_descriptors_track_outline_kinds_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "document-outline-landmark-page")
        .expect("document outline fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("document outline fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.section_landmark_descriptors, expected.section_landmark_descriptors,
        "section landmark descriptors should classify outline/landmark kinds and readiness blockers",
    );
    assert_eq!(
        actual.section_landmark_descriptors[1].outline_kind, "landmark-section",
        "navigation landmarks should remain visible as sectioning landmarks",
    );
    assert_eq!(
        actual.section_landmark_descriptors[3].landmark_block_reasons,
        Vec::<String>::new(),
        "headed articles should not be blocked",
    );
}

#[test]
fn browser_command_element_descriptor_metadata_tracks_activation_surfaces() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive element fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive element fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.command_elements, expected.command_elements,
        "command descriptors should preserve routed, ARIA, popover, and disclosure activation metadata",
    );
}

#[test]
fn browser_activation_descriptors_track_command_routes_and_state() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive activation fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive activation fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.activation_descriptors, expected.activation_descriptors,
        "activation descriptors should preserve command, popover, disclosure, ARIA, focus, and inline handler routing metadata",
    );
}

#[test]
fn browser_popover_descriptor_metadata_tracks_hosts_and_invokers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "popover-command-descriptor-page")
        .expect("popover command fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("popover command fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.popovers, expected.popovers,
        "popover descriptors should preserve host metadata and popovertarget/commandfor invoker relationships",
    );
}

#[test]
fn browser_popover_descriptors_track_hosts_invokers_and_actions() {
    let actual = parse_browser_document(
        r#"<button id=open popovertarget=panel popovertargetaction=show aria-expanded=false>Open</button>
           <button id=cmd command=toggle-popover commandfor=panel aria-expanded=true>Toggle</button>
           <div id=panel popover=manual aria-label="Panel">Panel copy</div>"#,
    )
    .expect("popover descriptor fixture should parse");

    assert_eq!(actual.popover_descriptors.len(), 1);
    let descriptor = &actual.popover_descriptors[0];
    assert_eq!(descriptor.popover_index, 1);
    assert_eq!(descriptor.element, "div");
    assert_eq!(descriptor.id.as_deref(), Some("panel"));
    assert_eq!(descriptor.role, "block");
    assert_eq!(descriptor.accessible_name.as_deref(), Some("Panel"));
    assert_eq!(descriptor.popover_mode, "manual");
    assert_eq!(descriptor.invoker_count, 2);
    assert_eq!(descriptor.invoker_ids, vec!["open", "cmd"]);
    assert_eq!(
        descriptor.invoker_actions,
        vec!["popover-show", "toggle-popover"]
    );
    assert_eq!(descriptor.invoker_aria_expanded, vec!["false", "true"]);
    assert_eq!(descriptor.focusable_invoker_count, 2);
    assert!(!descriptor.popover_blocked);
}

#[test]
fn browser_popover_descriptors_track_missing_and_invalid_blockers() {
    let actual = parse_browser_document(
        r#"<div id=bad popover=maybe>Bad</div>
           <div popover=manual>No id</div>
           <button id=bad-action popovertarget=bad popovertargetaction=launch tabindex=-1>Bad action</button>"#,
    )
    .expect("blocked popover descriptor fixture should parse");

    assert_eq!(actual.popover_descriptors.len(), 2);

    let bad = &actual.popover_descriptors[0];
    assert_eq!(bad.id.as_deref(), Some("bad"));
    assert_eq!(bad.popover_mode, "maybe");
    assert_eq!(bad.invoker_count, 1);
    assert_eq!(
        bad.popover_block_reasons,
        vec![
            "invalid-popover-mode",
            "non-focusable-invoker",
            "invalid-popover-target-action"
        ],
    );

    let missing_id = &actual.popover_descriptors[1];
    assert_eq!(missing_id.id, None);
    assert_eq!(
        missing_id.popover_block_reasons,
        vec!["missing-id", "missing-invokers"],
    );
}

#[test]
fn browser_aria_collection_descriptor_metadata_tracks_grouped_composites() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-collection-descriptor-page")
        .expect("ARIA collection fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA collection fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.aria_collections, expected.aria_collections,
        "ARIA collection descriptors should preserve composite roles, active descendants, item roles, and item states",
    );
    assert_eq!(
        actual.aria_collection_descriptors, expected.aria_collection_descriptors,
        "ARIA collection descriptor metadata should flatten composite inventory and readiness blockers",
    );
}

#[test]
fn browser_aria_collection_descriptors_track_selection_model_and_active_descendant() {
    let actual = parse_browser_document(
        r#"<body>
            <div id=menu role=menu aria-activedescendant=item-two>
              <button id=item-one role=menuitem>One</button>
              <button id=item-two role=menuitemcheckbox aria-checked=mixed>Two</button>
            </div>
            <div id=list role=listbox aria-multiselectable=true>
              <div id=alpha role=option aria-selected=true>Alpha</div>
              <div id=beta role=option aria-selected=true aria-disabled=true>Beta</div>
            </div>
        </body>"#,
    )
    .expect("ARIA collection descriptor fixture should parse");

    let menu = actual
        .aria_collections
        .iter()
        .find(|collection| collection.id.as_deref() == Some("menu"))
        .expect("menu collection should be present");
    assert_eq!(menu.item_roles, vec!["menuitem", "menuitemcheckbox"]);
    assert_eq!(menu.selection_mode, "single");
    assert!(menu.active_descendant_matches_item);

    let list = actual
        .aria_collections
        .iter()
        .find(|collection| collection.id.as_deref() == Some("list"))
        .expect("listbox collection should be present");
    assert_eq!(list.item_roles, vec!["option"]);
    assert_eq!(list.selection_mode, "multiple");
    assert!(!list.active_descendant_matches_item);
    assert_eq!(list.disabled_item_count, 1);
}

#[test]
fn browser_aria_collection_descriptors_track_selection_inventory_and_activedescendant() {
    let actual = parse_browser_document(
        r#"<body>
            <div id=menu role=menu aria-label="Actions" aria-activedescendant=item-two>
              <button id=item-one role=menuitem>One</button>
              <button id=item-two role=menuitemcheckbox aria-checked=mixed>Two</button>
            </div>
            <div id=list role=listbox aria-label="Choices" aria-multiselectable=true>
              <div id=alpha role=option aria-selected=true>Alpha</div>
              <div id=beta role=option aria-selected=true aria-disabled=true>Beta</div>
            </div>
        </body>"#,
    )
    .expect("ARIA collection descriptor fixture should parse");

    assert_eq!(actual.aria_collection_descriptors.len(), 2);
    let menu = &actual.aria_collection_descriptors[0];
    assert_eq!(menu.collection_index, 1);
    assert_eq!(menu.id.as_deref(), Some("menu"));
    assert_eq!(menu.collection_kind, "menu");
    assert_eq!(menu.accessible_name.as_deref(), Some("Actions"));
    assert_eq!(menu.item_count, 2);
    assert_eq!(menu.item_roles, vec!["menuitem", "menuitemcheckbox"]);
    assert_eq!(menu.checked_item_count, 1);
    assert_eq!(menu.selection_mode, "single");
    assert!(menu.active_descendant_matches_item);
    assert!(!menu.collection_blocked);

    let list = &actual.aria_collection_descriptors[1];
    assert_eq!(list.collection_kind, "listbox");
    assert_eq!(list.aria_multiselectable.as_deref(), Some("true"));
    assert_eq!(list.item_roles, vec!["option"]);
    assert_eq!(list.selected_item_count, 2);
    assert_eq!(list.disabled_item_count, 1);
    assert_eq!(list.selection_mode, "multiple");
    assert!(!list.collection_blocked);
}

#[test]
fn browser_aria_collection_descriptors_track_missing_and_unresolved_blockers() {
    let actual = parse_browser_document(
        r#"<div id=bad role=menu aria-activedescendant=ghost aria-owns=ghost></div>"#,
    )
    .expect("blocked ARIA collection descriptor fixture should parse");

    assert_eq!(actual.aria_collection_descriptors.len(), 1);
    let bad = &actual.aria_collection_descriptors[0];
    assert_eq!(bad.id.as_deref(), Some("bad"));
    assert_eq!(bad.item_count, 0);
    assert_eq!(
        bad.collection_block_reasons,
        vec![
            "missing-accessible-name",
            "missing-items",
            "unresolved-active-descendant",
            "unresolved-owned-items",
        ],
    );
}

#[test]
fn browser_aria_range_descriptor_metadata_tracks_value_widgets() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-range-descriptor-page")
        .expect("ARIA range fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA range fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.aria_ranges, expected.aria_ranges,
        "ARIA range descriptors should preserve value bounds, value text, orientation, and value widget states",
    );
}

#[test]
fn browser_aria_range_descriptors_track_value_completeness_focus_and_blockers() {
    let actual = parse_browser_document(
        r#"<body>
            <div id=volume role=slider aria-valuemin=0 aria-valuemax=10 aria-valuenow=7 aria-valuetext="7 of 10" tabindex=0></div>
            <div id=scroll role=scrollbar aria-valuemin=0 aria-valuemax=100 aria-disabled=true aria-readonly=true>Scroll</div>
        </body>"#,
    )
    .expect("ARIA range descriptor fixture should parse");

    let volume = actual
        .aria_ranges
        .iter()
        .find(|range| range.id.as_deref() == Some("volume"))
        .expect("volume range should be present");
    assert_eq!(
        volume.value_attribute_names,
        vec![
            "aria-valuemin",
            "aria-valuemax",
            "aria-valuenow",
            "aria-valuetext",
        ]
    );
    assert_eq!(volume.value_attribute_count, 4);
    assert!(volume.range_value_complete);
    assert!(volume.focusable);
    assert!(!volume.range_blocked);

    let scroll = actual
        .aria_ranges
        .iter()
        .find(|range| range.id.as_deref() == Some("scroll"))
        .expect("scrollbar range should be present");
    assert_eq!(
        scroll.value_attribute_names,
        vec!["aria-valuemin", "aria-valuemax"]
    );
    assert_eq!(scroll.value_attribute_count, 2);
    assert!(!scroll.range_value_complete);
    assert!(!scroll.focusable);
    assert!(scroll.range_blocked);
    assert_eq!(
        scroll.range_block_reasons,
        vec!["aria-disabled", "aria-readonly"]
    );
}

#[test]
fn browser_aria_live_region_descriptor_metadata_tracks_update_semantics() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-live-region-descriptor-page")
        .expect("ARIA live-region fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA live-region fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.aria_live_regions, expected.aria_live_regions,
        "ARIA live-region descriptors should preserve live politeness, busy/atomic/relevant flags, and implicit update semantics",
    );
}

#[test]
fn browser_aria_live_region_descriptors_track_attributes_assertive_and_blockers() {
    let actual = parse_browser_document(
        r#"<body>
            <div id=alert role=alert aria-busy=true>Error</div>
            <div id=clock role=timer aria-live=off>12:00</div>
            <p id=hidden role=status aria-hidden=true aria-atomic=true>Muted</p>
        </body>"#,
    )
    .expect("ARIA live-region descriptor fixture should parse");

    let alert = actual
        .aria_live_regions
        .iter()
        .find(|region| region.id.as_deref() == Some("alert"))
        .expect("alert live region should be present");
    assert_eq!(alert.live_attribute_names, vec!["aria-busy"]);
    assert_eq!(alert.live_attribute_count, 1);
    assert!(alert.assertive_update);
    assert!(!alert.live_region_blocked);

    let clock = actual
        .aria_live_regions
        .iter()
        .find(|region| region.id.as_deref() == Some("clock"))
        .expect("timer live region should be present");
    assert_eq!(clock.live_attribute_names, vec!["aria-live"]);
    assert!(!clock.assertive_update);
    assert!(clock.live_region_blocked);
    assert_eq!(clock.live_region_block_reasons, vec!["live-off"]);

    let hidden = actual
        .aria_live_regions
        .iter()
        .find(|region| region.id.as_deref() == Some("hidden"))
        .expect("hidden live region should be present");
    assert_eq!(
        hidden.live_attribute_names,
        vec!["aria-atomic", "aria-hidden"]
    );
    assert!(hidden.live_region_blocked);
    assert_eq!(hidden.live_region_block_reasons, vec!["aria-hidden"]);
}

#[test]
fn browser_form_validation_metadata_tracks_constraint_candidates() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");

    assert_eq!(
        actual.forms,
        case.expected.into_browser_document().forms,
        "form controls should preserve validation candidate metadata",
    );
}

#[test]
fn browser_form_validation_descriptor_metadata_tracks_constraint_attributes_and_barred_reasons() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form controls should preserve validation attributes and barred validation reasons",
    );
}

#[test]
fn browser_form_validation_control_descriptors_track_candidates_and_barred_controls() {
    let document = parse_browser_document(
        "<form id=signup>\
         <input id=email name=email type=email required minlength=3 maxlength=80>\
         <input id=age name=age type=number min=18 max=120 step=1>\
         <textarea id=bio name=bio readonly maxlength=200>About me</textarea>\
         <input id=token type=hidden name=token value=abc>\
         <fieldset disabled><legend>Legacy</legend><input id=legacy name=legacy required></fieldset>\
         <output id=preview name=preview for=\"email age\">Preview</output>\
         <input id=go type=image name=go src=go.png alt=Go>\
         </form>\
         <input id=external form=signup name=outside required>",
    )
    .expect("form validation descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("signup form should be summarized");
    let descriptors = &form.validation_controls;
    let ids: Vec<&str> = descriptors
        .iter()
        .filter_map(|control| control.id.as_deref())
        .collect();
    assert_eq!(
        ids,
        vec!["email", "age", "bio", "token", "legacy", "preview", "go", "external"]
    );

    assert!(descriptors[0].will_validate);
    assert!(descriptors[0].required);
    assert_eq!(
        descriptors[0].validation_attributes,
        vec!["required", "minlength", "maxlength"]
    );
    assert!(descriptors[1].will_validate);
    assert_eq!(
        descriptors[1].validation_attributes,
        vec!["min", "max", "step"]
    );
    assert!(!descriptors[2].will_validate);
    assert_eq!(
        descriptors[2].validation_barred_reason.as_deref(),
        Some("readonly")
    );
    assert_eq!(
        descriptors[3].validation_barred_reason.as_deref(),
        Some("input-type-hidden")
    );
    assert!(!descriptors[4].will_validate);
    assert!(descriptors[4].required);
    assert_eq!(
        descriptors[4].validation_barred_reason.as_deref(),
        Some("disabled")
    );
    assert_eq!(
        descriptors[5].validation_barred_reason.as_deref(),
        Some("output")
    );
    assert_eq!(
        descriptors[6].validation_barred_reason.as_deref(),
        Some("input-type-image")
    );
    assert_eq!(descriptors[7].form_owner.as_deref(), Some("signup"));
    assert!(descriptors[7].will_validate);
    assert_eq!(descriptors[7].validation_attributes, vec!["required"]);
}

#[test]
fn browser_form_validation_descriptors_track_flat_candidates_and_bypass_hints() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form validation descriptor fixture should parse");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.form_validation_descriptors, expected.form_validation_descriptors,
        "form validation descriptors should flatten validation candidates, barred controls, and submitter bypass hints",
    );
}

#[test]
fn browser_form_control_descriptors_track_flat_control_inventory() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual =
        parse_browser_document(&case.input).expect("form control descriptor fixture should parse");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.form_control_descriptors, expected.form_control_descriptors,
        "form control descriptors should flatten control state, labels, values, fieldsets, and blocker metadata",
    );
}

#[test]
fn browser_form_control_descriptors_track_state_values_and_labels() {
    let actual = parse_browser_document(
        "<form id=profile name=profile>\
         <label for=q>Query</label><input id=q name=q value=rust required list=suggestions>\
         <datalist id=suggestions><option value=Rust><option>HTML</datalist>\
         <select id=mode name=mode multiple><option value=web selected>Web<option value=cli>CLI</select>\
         <button id=go name=go>Go</button></form>\
         <input id=outside form=profile name=outside disabled>",
    )
    .expect("form control descriptor fixture should parse");

    assert_eq!(actual.form_control_descriptors.len(), 4);

    let query = &actual.form_control_descriptors[0];
    assert_eq!(query.element, "input");
    assert_eq!(query.id.as_deref(), Some("q"));
    assert_eq!(query.control_kind, "successful-control");
    assert_eq!(query.labels, vec!["Query"]);
    assert_eq!(query.label_count, 1);
    assert_eq!(query.value.as_deref(), Some("rust"));
    assert_eq!(query.submission_values, vec!["rust"]);
    assert_eq!(query.datalist_options, vec!["Rust", "HTML"]);
    assert!(query.required);
    assert!(query.successful);
    assert!(query.will_validate);
    assert!(!query.control_blocked);

    let mode = &actual.form_control_descriptors[1];
    assert_eq!(mode.element, "select");
    assert_eq!(mode.control_kind, "selection-control");
    assert!(mode.multiple);
    assert_eq!(mode.option_count, 2);
    assert_eq!(mode.selected_options, vec!["web"]);
    assert_eq!(mode.submission_values, vec!["web"]);

    let go = &actual.form_control_descriptors[2];
    assert_eq!(go.id.as_deref(), Some("go"));
    assert_eq!(go.control_kind, "submitter-control");
    assert_eq!(go.accessible_name.as_deref(), Some("Go"));

    let outside = &actual.form_control_descriptors[3];
    assert_eq!(outside.form_owner.as_deref(), Some("profile"));
    assert_eq!(outside.control_kind, "blocked-control");
    assert!(outside.disabled);
    assert_eq!(
        outside.control_block_reasons,
        vec!["disabled", "validation-barred:disabled"]
    );
}

#[test]
fn browser_form_control_descriptors_track_missing_names_and_blockers() {
    let actual = parse_browser_document(
        "<form id=survey>\
         <fieldset id=choices disabled><legend>Choices</legend>\
         <input id=maybe type=checkbox name=maybe></fieldset>\
         <textarea id=notes readonly>Read only</textarea>\
         <input id=orphan value=no-name>\
         </form>",
    )
    .expect("blocked form control descriptor fixture should parse");

    assert_eq!(actual.form_control_descriptors.len(), 3);

    let maybe = &actual.form_control_descriptors[0];
    assert_eq!(maybe.id.as_deref(), Some("maybe"));
    assert_eq!(maybe.control_kind, "blocked-control");
    assert_eq!(maybe.fieldset_ids, vec!["choices"]);
    assert_eq!(maybe.fieldset_legends, vec!["Choices"]);
    assert_eq!(
        maybe.control_block_reasons,
        vec!["disabled", "unchecked-choice", "validation-barred:disabled"]
    );

    let notes = &actual.form_control_descriptors[1];
    assert_eq!(notes.element, "textarea");
    assert_eq!(
        notes.control_block_reasons,
        vec!["missing-name", "readonly", "validation-barred:readonly"]
    );

    let orphan = &actual.form_control_descriptors[2];
    assert_eq!(orphan.id.as_deref(), Some("orphan"));
    assert_eq!(orphan.control_kind, "blocked-control");
    assert_eq!(orphan.control_block_reasons, vec!["missing-name"]);
}

#[test]
fn browser_form_output_descriptor_metadata_tracks_for_references_and_labels() {
    let document = parse_browser_document(
        "<form id=calc>\
         <label for=sum>Sum</label>\
         <input id=a name=a value=1>\
         <input id=b name=b value=2>\
         <p id=hint>Total of inputs</p>\
         <output id=sum name=sum for=\"a b missing\" aria-describedby=hint>3</output>\
         <fieldset disabled><legend>Preview</legend><output id=disabled name=preview for=a>Disabled output</output></fieldset>\
         </form>\
         <output id=external form=calc name=external for=sum aria-label=External>External</output>",
    )
    .expect("form output descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("calc form should be summarized");
    let ids: Vec<&str> = form
        .outputs
        .iter()
        .filter_map(|output| output.id.as_deref())
        .collect();
    assert_eq!(ids, vec!["sum", "disabled", "external"]);

    let sum = &form.outputs[0];
    assert_eq!(sum.name.as_deref(), Some("sum"));
    assert_eq!(sum.labels, vec!["Sum"]);
    assert_eq!(sum.accessible_name.as_deref(), Some("Sum"));
    assert_eq!(
        sum.accessible_description.as_deref(),
        Some("Total of inputs")
    );
    assert_eq!(sum.for_tokens, vec!["a", "b", "missing"]);
    assert_eq!(sum.for_control_ids, vec!["a", "b"]);
    assert_eq!(sum.for_control_names, vec!["a", "b"]);
    assert_eq!(sum.for_control_types, vec!["text", "text"]);
    assert_eq!(sum.value.as_deref(), Some("3"));
    assert_eq!(sum.text, "3");
    assert!(!sum.disabled);
    assert!(!sum.will_validate);
    assert_eq!(sum.validation_barred_reason.as_deref(), Some("output"));

    let disabled = &form.outputs[1];
    assert_eq!(disabled.name.as_deref(), Some("preview"));
    assert!(disabled.disabled);
    assert_eq!(
        disabled.validation_barred_reason.as_deref(),
        Some("disabled")
    );
    assert_eq!(disabled.for_control_ids, vec!["a"]);
    assert_eq!(disabled.for_control_names, vec!["a"]);

    let external = &form.outputs[2];
    assert_eq!(external.form_owner.as_deref(), Some("calc"));
    assert_eq!(external.accessible_name.as_deref(), Some("External"));
    assert_eq!(external.for_tokens, vec!["sum"]);
    assert_eq!(external.for_control_ids, vec!["sum"]);
    assert_eq!(external.for_control_names, vec!["sum"]);
    assert_eq!(external.for_control_types, vec!["output"]);
    assert_eq!(external.value.as_deref(), Some("External"));
}

#[test]
fn browser_form_object_descriptor_metadata_tracks_embedded_controls_and_params() {
    let document = parse_browser_document(
        "<base href=\"https://example.test/media/\">\
         <form id=player>\
         <p id=movie-help>Legacy plugin fallback</p>\
         <fieldset id=plugins><legend>Plugins</legend>\
         <object id=movie name=movie data=movie.swf type=\"application/x-shockwave-flash\" \
             width=400 height=300 usemap=#movie-map aria-label=\"Movie plugin\" aria-describedby=movie-help>\
             <param name=autoplay value=false><param name=quality value=high>Fallback player\
         </object>\
         </fieldset>\
         <input id=token name=token value=ok>\
         </form>\
         <object id=external name=external form=player data=/external.bin type=\"application/octet-stream\" aria-label=External>External fallback</object>\
         <object id=outside name=outside data=outside.bin>Outside fallback</object>",
    )
    .expect("form object descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("player form should be summarized");

    let object_ids: Vec<&str> = form
        .object_controls
        .iter()
        .filter_map(|object| object.id.as_deref())
        .collect();
    assert_eq!(object_ids, vec!["movie", "external"]);

    let movie = &form.object_controls[0];
    assert_eq!(movie.name.as_deref(), Some("movie"));
    assert_eq!(movie.accessible_name.as_deref(), Some("Movie plugin"));
    assert_eq!(
        movie.accessible_description.as_deref(),
        Some("Legacy plugin fallback")
    );
    assert_eq!(movie.data.as_deref(), Some("movie.swf"));
    assert_eq!(
        movie.resolved_data.as_deref(),
        Some("https://example.test/media/movie.swf")
    );
    assert_eq!(
        movie.type_hint.as_deref(),
        Some("application/x-shockwave-flash")
    );
    assert_eq!(movie.width.as_deref(), Some("400"));
    assert_eq!(movie.height.as_deref(), Some("300"));
    assert_eq!(movie.usemap.as_deref(), Some("#movie-map"));
    assert_eq!(movie.fallback_text, "Fallback player");
    assert_eq!(movie.params.len(), 2);
    assert_eq!(movie.params[0].name.as_deref(), Some("autoplay"));
    assert_eq!(movie.params[0].value.as_deref(), Some("false"));
    assert_eq!(movie.params[1].name.as_deref(), Some("quality"));
    assert_eq!(movie.params[1].value.as_deref(), Some("high"));

    let external = &form.object_controls[1];
    assert_eq!(external.form_owner.as_deref(), Some("player"));
    assert_eq!(external.name.as_deref(), Some("external"));
    assert_eq!(external.accessible_name.as_deref(), Some("External"));
    assert_eq!(
        external.resolved_data.as_deref(),
        Some("https://example.test/external.bin")
    );

    assert_eq!(form.fieldsets[0].control_ids, vec!["movie"]);
    assert_eq!(form.fieldsets[0].control_names, vec!["movie"]);
    assert_eq!(form.controls.len(), 1);
    assert_eq!(form.controls[0].id.as_deref(), Some("token"));
    assert_eq!(form.successful_controls.len(), 1);
    assert_eq!(form.successful_controls[0].name, "token");
    assert_eq!(form.validation_controls.len(), 1);
    assert_eq!(form.validation_controls[0].id.as_deref(), Some("token"));
}

#[test]
fn browser_form_fieldset_metadata_disables_descendant_controls() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form controls should reflect disabled fieldset ancestry",
    );
}

#[test]
fn browser_form_fieldset_group_metadata_tracks_legends_disabled_state_and_controls() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form summaries should preserve fieldset legends, disabled state, and grouped controls",
    );
}

#[test]
fn browser_form_successful_control_metadata_tracks_submission_values() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "catalog-form-table-page")
        .expect("catalog form fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("catalog form fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form controls should preserve successful-control and submission value metadata",
    );
}

#[test]
fn browser_form_successful_control_descriptor_metadata_tracks_submission_entries() {
    let document = parse_browser_document(
        "<form id=checkout>\
         <input id=item name=item value=book>\
         <input id=gift type=checkbox name=gift value=yes checked>\
         <input id=skip type=checkbox name=skip value=yes>\
         <input id=disabled name=disabled value=no disabled>\
         <select id=shipping name=shipping><option value=ground>Ground<option value=air selected>Air</select>\
         <textarea id=note name=note>Leave at desk</textarea>\
         <input id=file type=file name=upload>\
         <button name=go>Submit</button></form>\
         <input id=outside form=checkout name=outside value=external>",
    )
    .expect("form successful-control descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("checkout form should be summarized");
    let names: Vec<&str> = form
        .successful_controls
        .iter()
        .map(|control| control.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["item", "gift", "shipping", "note", "upload", "outside"]
    );
    assert_eq!(form.successful_controls[0].id.as_deref(), Some("item"));
    assert_eq!(form.successful_controls[0].control_type, "text");
    assert_eq!(form.successful_controls[0].submission_values, vec!["book"]);
    assert_eq!(form.successful_controls[1].control_type, "checkbox");
    assert_eq!(form.successful_controls[1].submission_values, vec!["yes"]);
    assert_eq!(form.successful_controls[2].control_type, "select");
    assert_eq!(form.successful_controls[2].submission_values, vec!["air"]);
    assert_eq!(form.successful_controls[3].control_type, "textarea");
    assert_eq!(
        form.successful_controls[3].submission_values,
        vec!["Leave at desk"]
    );
    assert_eq!(form.successful_controls[4].control_type, "file");
    assert!(form.successful_controls[4].submission_values.is_empty());
    assert_eq!(
        form.successful_controls[5].form_owner.as_deref(),
        Some("checkout")
    );
    assert_eq!(
        form.successful_controls[5].submission_values,
        vec!["external"]
    );
}

#[test]
fn browser_form_submission_descriptors_track_flat_successful_controls_and_submitters() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form submission descriptor fixture should parse");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.form_submission_descriptors, expected.form_submission_descriptors,
        "form submission descriptors should flatten successful controls and submitter routing",
    );
}

#[test]
fn browser_form_association_descriptors_track_flat_owner_and_label_links() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form association descriptor fixture should parse");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.form_association_descriptors, expected.form_association_descriptors,
        "form association descriptors should flatten owners, labels, fieldsets, datalists, and outputs",
    );
}

#[test]
fn browser_form_autofill_descriptors_track_flat_autocomplete_hints_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual =
        parse_browser_document(&case.input).expect("form autofill descriptor fixture should parse");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.form_autofill_descriptors, expected.form_autofill_descriptors,
        "form autofill descriptors should flatten autocomplete hints and blockers",
    );
}

#[test]
fn browser_form_reset_descriptors_track_flat_resetters_and_controls() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual =
        parse_browser_document(&case.input).expect("form reset descriptor fixture should parse");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.form_reset_descriptors, expected.form_reset_descriptors,
        "form reset descriptors should flatten resettable controls and resetter controls",
    );
}

#[test]
fn browser_form_hidden_descriptor_metadata_tracks_hidden_entries_and_form_owners() {
    let document = parse_browser_document(
        "<form id=session>\
         <input id=csrf type=hidden name=csrf value=token autocomplete=\"section-auth one-time-code\">\
         <input id=charset type=hidden name=_charset_ value=utf-8>\
         <input id=disabled-token type=hidden name=disabled value=no disabled>\
         </form>\
         <input id=external type=hidden form=session name=outside value=external>",
    )
    .expect("form hidden descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("session form should be summarized");
    let ids: Vec<&str> = form
        .hidden_controls
        .iter()
        .filter_map(|hidden| hidden.id.as_deref())
        .collect();
    assert_eq!(ids, vec!["csrf", "charset", "disabled-token", "external"]);

    let csrf = &form.hidden_controls[0];
    assert_eq!(csrf.name.as_deref(), Some("csrf"));
    assert_eq!(csrf.value.as_deref(), Some("token"));
    assert_eq!(
        csrf.autocomplete.as_deref(),
        Some("section-auth one-time-code")
    );
    assert_eq!(
        csrf.autocomplete_tokens,
        vec!["section-auth", "one-time-code"]
    );
    assert!(csrf.successful);
    assert_eq!(csrf.submission_values, vec!["token"]);
    assert!(!csrf.will_validate);
    assert_eq!(
        csrf.validation_barred_reason.as_deref(),
        Some("input-type-hidden")
    );

    let charset = &form.hidden_controls[1];
    assert_eq!(charset.name.as_deref(), Some("_charset_"));
    assert_eq!(charset.value.as_deref(), Some("utf-8"));
    assert!(charset.successful);
    assert_eq!(charset.submission_values, vec!["utf-8"]);

    let disabled = &form.hidden_controls[2];
    assert_eq!(disabled.name.as_deref(), Some("disabled"));
    assert!(disabled.disabled);
    assert!(!disabled.successful);
    assert!(disabled.submission_values.is_empty());
    assert_eq!(
        disabled.validation_barred_reason.as_deref(),
        Some("disabled")
    );

    let external = &form.hidden_controls[3];
    assert_eq!(external.form_owner.as_deref(), Some("session"));
    assert_eq!(external.name.as_deref(), Some("outside"));
    assert_eq!(external.submission_values, vec!["external"]);
}

#[test]
fn browser_form_button_descriptor_metadata_tracks_submitters_and_button_controls() {
    let document = parse_browser_document(
        "<base href=\"https://example.test/forms/index.html\" target=_base>\
         <form id=actions action=submit method=post target=_form novalidate>\
         <button id=save name=save value=s type=submit formaction=save formenctype=text/plain \
             formmethod=get formtarget=_save formnovalidate autofocus>Save</button>\
         <button id=reset type=reset name=reset value=r>Reset</button>\
         <input id=plain type=button name=plain value=Plain>\
         <input id=image type=image name=img src=go.png alt=Image width=20 height=10>\
         <button id=disabled name=disabled disabled>Disabled</button>\
         </form>\
         <button id=external form=actions type=submit name=outside formtarget=_outside>Outside</button>",
    )
    .expect("form button descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("actions form should be summarized");
    let ids: Vec<&str> = form
        .buttons
        .iter()
        .filter_map(|button| button.id.as_deref())
        .collect();
    assert_eq!(
        ids,
        vec!["save", "reset", "plain", "image", "disabled", "external"]
    );

    let save = &form.buttons[0];
    assert!(save.submitter);
    assert!(save.autofocus);
    assert_eq!(save.accessible_name.as_deref(), Some("Save"));
    assert_eq!(save.action.as_deref(), Some("save"));
    assert_eq!(
        save.resolved_action.as_deref(),
        Some("https://example.test/forms/save")
    );
    assert_eq!(save.method, "get");
    assert_eq!(save.enctype.as_deref(), Some("text/plain"));
    assert_eq!(save.target.as_deref(), Some("_save"));
    assert_eq!(save.effective_target.as_deref(), Some("_save"));
    assert!(save.novalidate);
    assert_eq!(save.value.as_deref(), Some("s"));
    assert_eq!(save.text, "Save");

    let reset = &form.buttons[1];
    assert_eq!(reset.control_type, "reset");
    assert!(!reset.submitter);
    assert_eq!(reset.method, "post");
    assert_eq!(reset.text, "Reset");

    let plain = &form.buttons[2];
    assert_eq!(plain.control_type, "button");
    assert!(!plain.submitter);
    assert_eq!(plain.value.as_deref(), Some("Plain"));
    assert!(plain.text.is_empty());

    let image = &form.buttons[3];
    assert_eq!(image.control_type, "image");
    assert!(image.submitter);
    assert_eq!(image.accessible_name.as_deref(), Some("Image"));
    assert_eq!(
        image.resolved_action.as_deref(),
        Some("https://example.test/forms/submit")
    );
    assert_eq!(image.effective_target.as_deref(), Some("_form"));
    assert_eq!(image.src.as_deref(), Some("go.png"));
    assert_eq!(
        image.resolved_src.as_deref(),
        Some("https://example.test/forms/go.png")
    );
    assert_eq!(image.width.as_deref(), Some("20"));
    assert_eq!(image.height.as_deref(), Some("10"));

    let disabled = &form.buttons[4];
    assert!(disabled.disabled);
    assert!(!disabled.submitter);
    assert!(!disabled.novalidate);

    let external = &form.buttons[5];
    assert_eq!(external.form_owner.as_deref(), Some("actions"));
    assert!(external.submitter);
    assert_eq!(external.effective_target.as_deref(), Some("_outside"));
    assert_eq!(external.text, "Outside");
}

#[test]
fn browser_form_descriptor_metadata_tracks_accept_and_autocomplete_tokens() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "form metadata should preserve tokenized accept-charset, autocomplete, and file accept descriptors",
    );
}

#[test]
fn browser_text_control_descriptor_metadata_tracks_editing_and_layout_hints() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "text controls should preserve spellcheck, autocorrect, rows, cols, and wrapping hints",
    );
}

#[test]
fn browser_form_text_entry_descriptor_metadata_tracks_textual_controls_and_editing_hints() {
    let document = parse_browser_document(
        "<form id=profile>\
         <label for=email>Email</label>\
         <input id=email type=email name=email placeholder=Email required \
             autocomplete=\"section-contact email\" inputmode=email minlength=3 maxlength=80 size=30>\
         <input id=search type=search name=q value=rust autocapitalize=words \
             enterkeyhint=search dirname=q.dir spellcheck=false autocorrect=off list=suggestions>\
         <datalist id=suggestions><option value=Rust><option value=HTML label=Markup></datalist>\
         <textarea id=bio name=bio rows=4 cols=40 wrap=hard readonly maxlength=200>About me</textarea>\
         <input id=age type=number name=age value=42 min=18 max=120 step=1>\
         <input id=check type=checkbox name=check checked>\
         </form>\
         <textarea id=external form=profile name=outside placeholder=Outside>External note</textarea>",
    )
    .expect("form text-entry descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("profile form should be summarized");
    let ids: Vec<&str> = form
        .text_entries
        .iter()
        .filter_map(|entry| entry.id.as_deref())
        .collect();
    assert_eq!(ids, vec!["email", "search", "bio", "age", "external"]);

    let email = &form.text_entries[0];
    assert_eq!(email.control_type, "email");
    assert_eq!(email.labels, vec!["Email"]);
    assert_eq!(email.accessible_name.as_deref(), Some("Email"));
    assert_eq!(email.placeholder.as_deref(), Some("Email"));
    assert_eq!(email.autocomplete_tokens, vec!["section-contact", "email"]);
    assert_eq!(email.inputmode.as_deref(), Some("email"));
    assert_eq!(email.minlength.as_deref(), Some("3"));
    assert_eq!(email.maxlength.as_deref(), Some("80"));
    assert_eq!(email.size.as_deref(), Some("30"));
    assert!(email.required);
    assert!(email.will_validate);
    assert_eq!(
        email.validation_attributes,
        vec!["required", "minlength", "maxlength"]
    );

    let search = &form.text_entries[1];
    assert_eq!(search.control_type, "search");
    assert_eq!(search.value.as_deref(), Some("rust"));
    assert_eq!(search.autocapitalize.as_deref(), Some("words"));
    assert_eq!(search.enterkeyhint.as_deref(), Some("search"));
    assert_eq!(search.dirname.as_deref(), Some("q.dir"));
    assert_eq!(search.spellcheck.as_deref(), Some("false"));
    assert_eq!(search.autocorrect.as_deref(), Some("off"));
    assert_eq!(search.list.as_deref(), Some("suggestions"));
    assert_eq!(search.datalist_options, vec!["Rust", "HTML"]);

    let bio = &form.text_entries[2];
    assert_eq!(bio.control_type, "textarea");
    assert_eq!(bio.text, "About me");
    assert_eq!(bio.rows.as_deref(), Some("4"));
    assert_eq!(bio.cols.as_deref(), Some("40"));
    assert_eq!(bio.wrap.as_deref(), Some("hard"));
    assert!(bio.readonly);
    assert!(!bio.will_validate);
    assert_eq!(bio.validation_barred_reason.as_deref(), Some("readonly"));

    let age = &form.text_entries[3];
    assert_eq!(age.control_type, "number");
    assert_eq!(age.min.as_deref(), Some("18"));
    assert_eq!(age.max.as_deref(), Some("120"));
    assert_eq!(age.step.as_deref(), Some("1"));

    let external = &form.text_entries[4];
    assert_eq!(external.form_owner.as_deref(), Some("profile"));
    assert_eq!(external.name.as_deref(), Some("outside"));
    assert_eq!(external.placeholder.as_deref(), Some("Outside"));
    assert_eq!(external.text, "External note");
}

#[test]
fn browser_form_choice_descriptor_metadata_tracks_checkbox_radio_state_and_groups() {
    let document = parse_browser_document(
        "<form id=prefs>\
         <label><input id=news type=checkbox name=news value=yes checked required>Newsletter</label>\
         <input id=updates type=checkbox name=updates>\
         <label for=plan-basic>Basic</label>\
         <input id=plan-basic type=radio name=plan value=basic checked required>\
         <input id=plan-pro type=radio name=plan value=pro>\
         <fieldset disabled><label><input id=legacy type=radio name=legacy value=old checked>Legacy</label></fieldset>\
         </form>\
         <input id=external type=radio form=prefs name=plan value=outside>",
    )
    .expect("form choice descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("prefs form should be summarized");
    let ids: Vec<&str> = form
        .choice_controls
        .iter()
        .filter_map(|choice| choice.id.as_deref())
        .collect();
    assert_eq!(
        ids,
        vec![
            "news",
            "updates",
            "plan-basic",
            "plan-pro",
            "legacy",
            "external"
        ]
    );

    let news = &form.choice_controls[0];
    assert_eq!(news.control_type, "checkbox");
    assert_eq!(news.labels, vec!["Newsletter"]);
    assert_eq!(news.accessible_name.as_deref(), Some("Newsletter"));
    assert_eq!(news.value.as_deref(), Some("yes"));
    assert!(news.checked);
    assert!(news.required);
    assert!(news.group_required);
    assert!(news.successful);
    assert_eq!(news.submission_values, vec!["yes"]);
    assert_eq!(news.group_checked_ids, vec!["news"]);
    assert_eq!(news.group_checked_values, vec!["yes"]);
    assert_eq!(news.validation_attributes, vec!["required"]);

    let updates = &form.choice_controls[1];
    assert_eq!(updates.control_type, "checkbox");
    assert_eq!(updates.value.as_deref(), Some("on"));
    assert!(!updates.checked);
    assert!(!updates.successful);
    assert!(updates.submission_values.is_empty());
    assert!(updates.group_checked_ids.is_empty());
    assert!(updates.group_checked_values.is_empty());

    let plan_basic = &form.choice_controls[2];
    assert_eq!(plan_basic.control_type, "radio");
    assert_eq!(plan_basic.labels, vec!["Basic"]);
    assert_eq!(plan_basic.group_name.as_deref(), Some("plan"));
    assert!(plan_basic.checked);
    assert!(plan_basic.required);
    assert!(plan_basic.group_required);
    assert_eq!(plan_basic.group_checked_ids, vec!["plan-basic"]);
    assert_eq!(plan_basic.group_checked_values, vec!["basic"]);

    let plan_pro = &form.choice_controls[3];
    assert_eq!(plan_pro.control_type, "radio");
    assert_eq!(plan_pro.group_name.as_deref(), Some("plan"));
    assert!(!plan_pro.required);
    assert!(plan_pro.group_required);
    assert_eq!(plan_pro.group_checked_ids, vec!["plan-basic"]);
    assert_eq!(plan_pro.group_checked_values, vec!["basic"]);

    let legacy = &form.choice_controls[4];
    assert_eq!(legacy.control_type, "radio");
    assert!(legacy.checked);
    assert!(legacy.disabled);
    assert!(!legacy.successful);
    assert_eq!(legacy.validation_barred_reason.as_deref(), Some("disabled"));
    assert_eq!(legacy.group_checked_values, vec!["old"]);

    let external = &form.choice_controls[5];
    assert_eq!(external.form_owner.as_deref(), Some("prefs"));
    assert_eq!(external.group_name.as_deref(), Some("plan"));
    assert_eq!(external.group_checked_ids, vec!["plan-basic"]);
    assert_eq!(external.group_checked_values, vec!["basic"]);
}

#[test]
fn browser_form_file_descriptor_metadata_tracks_upload_controls_and_hints() {
    let document = parse_browser_document(
        "<form id=uploads enctype=multipart/form-data>\
         <label for=avatar>Avatar</label>\
         <input id=avatar type=file name=avatar accept=\"image/png, image/jpeg, .webp\" capture=user multiple required>\
         <label>Attachment<input id=attachment type=file name=attachment disabled></label>\
         </form>\
         <input id=external type=file form=uploads name=outside accept=\"application/pdf\">",
    )
    .expect("form file descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("uploads form should be summarized");
    let ids: Vec<&str> = form
        .file_controls
        .iter()
        .filter_map(|file| file.id.as_deref())
        .collect();
    assert_eq!(ids, vec!["avatar", "attachment", "external"]);

    let avatar = &form.file_controls[0];
    assert_eq!(avatar.name.as_deref(), Some("avatar"));
    assert_eq!(avatar.labels, vec!["Avatar"]);
    assert_eq!(avatar.accessible_name.as_deref(), Some("Avatar"));
    assert_eq!(
        avatar.accept.as_deref(),
        Some("image/png, image/jpeg, .webp")
    );
    assert_eq!(
        avatar.accept_tokens,
        vec!["image/png", "image/jpeg", ".webp"]
    );
    assert_eq!(avatar.capture.as_deref(), Some("user"));
    assert!(avatar.multiple);
    assert!(avatar.required);
    assert!(avatar.successful);
    assert!(avatar.submission_values.is_empty());
    assert!(avatar.will_validate);
    assert_eq!(avatar.validation_attributes, vec!["required"]);

    let attachment = &form.file_controls[1];
    assert_eq!(attachment.labels, vec!["Attachment"]);
    assert!(attachment.disabled);
    assert!(!attachment.successful);
    assert!(!attachment.will_validate);
    assert_eq!(
        attachment.validation_barred_reason.as_deref(),
        Some("disabled")
    );

    let external = &form.file_controls[2];
    assert_eq!(external.form_owner.as_deref(), Some("uploads"));
    assert_eq!(external.accept.as_deref(), Some("application/pdf"));
    assert_eq!(external.accept_tokens, vec!["application/pdf"]);
    assert!(external.successful);
}

#[test]
fn browser_select_option_descriptor_metadata_tracks_values_labels_groups_and_state() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form accessibility fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.forms, expected.forms,
        "select controls should preserve option values, labels, optgroup labels, selected state, and disabled state",
    );
}

#[test]
fn browser_form_select_descriptor_metadata_tracks_option_items_and_selection_state() {
    let document = parse_browser_document(
        "<form id=survey><label for=topic>Topic</label>\
         <select id=topic name=topic required multiple size=4>\
         <optgroup label=Primary><option value=rust selected>Rust<option value=html label=HTML>Markup</optgroup>\
         <optgroup label=Archived disabled><option value=mosaic>Mosaic</optgroup>\
         </select></form>\
         <select id=external form=survey name=external><option>One<option selected>Two</select>",
    )
    .expect("form select descriptor fixture should parse");

    let form = document
        .forms
        .first()
        .expect("survey form should be summarized");
    assert_eq!(form.selects.len(), 2);

    let topic = &form.selects[0];
    assert_eq!(topic.id.as_deref(), Some("topic"));
    assert_eq!(topic.name.as_deref(), Some("topic"));
    assert_eq!(topic.labels, vec!["Topic"]);
    assert_eq!(topic.accessible_name.as_deref(), Some("Topic"));
    assert!(topic.required);
    assert!(topic.multiple);
    assert_eq!(topic.size.as_deref(), Some("4"));
    assert_eq!(topic.value.as_deref(), Some("rust"));
    assert_eq!(topic.selected_options, vec!["rust"]);
    assert_eq!(topic.text, "Rust Markup Mosaic");
    assert_eq!(topic.options.len(), 3);
    assert_eq!(topic.options[0].value, "rust");
    assert_eq!(topic.options[0].group_label.as_deref(), Some("Primary"));
    assert!(topic.options[0].selected);
    assert_eq!(topic.options[1].label.as_deref(), Some("HTML"));
    assert_eq!(topic.options[1].text, "Markup");
    assert_eq!(topic.options[2].value, "mosaic");
    assert_eq!(topic.options[2].group_label.as_deref(), Some("Archived"));
    assert!(topic.options[2].disabled);

    let external = &form.selects[1];
    assert_eq!(external.id.as_deref(), Some("external"));
    assert_eq!(external.form_owner.as_deref(), Some("survey"));
    assert_eq!(external.selected_options, vec!["Two"]);
    assert_eq!(external.options[1].value, "Two");
}

#[test]
fn browser_embedded_context_metadata_tracks_frame_object_embed_and_srcdoc() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "embedded-resource-page")
        .expect("embedded resource fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("embedded resource fixture should parse into browser document facts");

    assert_eq!(
        actual.embedded_contexts,
        case.expected.into_browser_document().embedded_contexts,
        "embedded contexts should preserve frame, object, embed, and srcdoc-only iframe metadata",
    );
}

#[test]
fn browser_embedded_policy_metadata_tracks_fetchpriority_and_csp() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "embedded-resource-page")
        .expect("embedded resource fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("embedded resource fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.resources, expected.resources,
        "frame resources should preserve fetch priority and CSP policy metadata",
    );
    assert_eq!(
        actual.embedded_contexts, expected.embedded_contexts,
        "embedded contexts should preserve fetch priority and CSP policy metadata",
    );
}

#[test]
fn browser_media_policy_metadata_tracks_controls_and_remote_playback_hints() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "media-playback-poster-page")
        .expect("media playback fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("media playback fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.resources, expected.resources,
        "media resources should preserve cross-origin fetch metadata",
    );
    assert_eq!(
        actual.media, expected.media,
        "media summaries should preserve controlslist and remote playback policy metadata",
    );
}

#[test]
fn browser_media_descriptor_metadata_tracks_sources_and_tracks() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "media-source-track-page")
        .expect("media source and track fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("media source and track fixture should parse into browser document facts");
    let expected = case.expected.into_browser_document();

    assert_eq!(
        actual.resources, expected.resources,
        "media child resources should preserve source candidates and track metadata",
    );
    assert_eq!(
        actual.media, expected.media,
        "media summaries should preserve nested source and track descriptors",
    );
}

#[test]
fn browser_media_resource_descriptors_track_source_and_text_track_candidates() {
    let summary = parse_browser_document(
        "<base href=\"https://example.test/watch/\">\
         <body><video controls>\
           <source src=movie.webm type=video/webm media=\"(min-width: 700px)\">\
           <source src=movie.mp4 type=video/mp4>\
           <track kind=captions src=captions.vtt srclang=en label=English default>\
           <track kind=metadata src=chapters.vtt label=Chapters>\
         </video>\
         <audio controls><source src=theme.ogg type=audio/ogg></audio>",
    )
    .expect("media resource descriptors fixture should parse");

    assert_eq!(summary.media_resource_descriptors.len(), 5);

    let webm = &summary.media_resource_descriptors[0];
    assert_eq!(webm.media_index, 1);
    assert_eq!(webm.media_kind, "video");
    assert_eq!(webm.element, "source");
    assert_eq!(webm.resource_kind, "source");
    assert_eq!(webm.src.as_deref(), Some("movie.webm"));
    assert_eq!(
        webm.resolved_src.as_deref(),
        Some("https://example.test/watch/movie.webm")
    );
    assert_eq!(webm.type_hint.as_deref(), Some("video/webm"));
    assert_eq!(webm.media.as_deref(), Some("(min-width: 700px)"));
    assert_eq!(webm.candidate_kind, "source-candidate");
    assert!(!webm.media_resource_blocked);

    let captions = &summary.media_resource_descriptors[2];
    assert_eq!(captions.element, "track");
    assert_eq!(captions.resource_kind, "track");
    assert_eq!(captions.track_kind.as_deref(), Some("captions"));
    assert_eq!(captions.srclang.as_deref(), Some("en"));
    assert_eq!(captions.label.as_deref(), Some("English"));
    assert!(captions.default_track);
    assert_eq!(captions.candidate_kind, "default-text-track");
    assert!(!captions.media_resource_blocked);

    let metadata = &summary.media_resource_descriptors[3];
    assert_eq!(metadata.track_kind.as_deref(), Some("metadata"));
    assert_eq!(metadata.label.as_deref(), Some("Chapters"));
    assert_eq!(metadata.candidate_kind, "text-track");

    let audio_source = &summary.media_resource_descriptors[4];
    assert_eq!(audio_source.media_index, 2);
    assert_eq!(audio_source.media_kind, "audio");
    assert_eq!(audio_source.src.as_deref(), Some("theme.ogg"));
}

#[test]
fn browser_media_resource_descriptors_track_missing_sources_and_labels() {
    let summary = parse_browser_document(
        "<video controls>\
           <source type=video/mp4>\
           <track kind=captions src=captions.vtt>\
           <track label=Descriptions>\
         </video>",
    )
    .expect("blocked media resource descriptors fixture should parse");

    assert_eq!(summary.media_resource_descriptors.len(), 3);

    let source = &summary.media_resource_descriptors[0];
    assert_eq!(source.element, "source");
    assert_eq!(source.candidate_kind, "blocked-source");
    assert!(source.media_resource_blocked);
    assert_eq!(source.media_resource_block_reasons, vec!["missing-src"]);

    let captions = &summary.media_resource_descriptors[1];
    assert_eq!(captions.element, "track");
    assert_eq!(captions.track_kind.as_deref(), Some("captions"));
    assert_eq!(
        captions.media_resource_block_reasons,
        vec!["missing-label", "missing-srclang"]
    );

    let descriptions = &summary.media_resource_descriptors[2];
    assert_eq!(descriptions.track_kind.as_deref(), Some("subtitles"));
    assert_eq!(
        descriptions.media_resource_block_reasons,
        vec!["missing-src", "missing-srclang"]
    );
}

#[test]
fn browser_interactive_element_metadata_tracks_focus_editing_and_commands() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive element fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive element fixture should parse into browser document facts");

    assert_eq!(
        actual.interactive_elements,
        case.expected.into_browser_document().interactive_elements,
        "interactive elements should preserve focus, editing, popover, command, hidden, and event metadata",
    );
}

#[test]
fn browser_focus_navigation_descriptors_track_focus_order_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive focus-navigation fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive focus-navigation fixture should parse into browser document facts");

    assert_eq!(
        actual.focus_navigation_descriptors,
        case.expected.into_browser_document().focus_navigation_descriptors,
        "focus-navigation descriptors should preserve focus order, programmatic focus, editing hosts, access keys, and blocked focus reasons",
    );
}

#[test]
fn browser_keyboard_interaction_descriptors_track_shortcuts_handlers_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive keyboard fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive keyboard fixture should parse into browser document facts");

    assert_eq!(
        actual.keyboard_interaction_descriptors,
        case.expected
            .into_browser_document()
            .keyboard_interaction_descriptors,
        "keyboard-interaction descriptors should preserve access keys, aria shortcuts, keyboard handlers, focus order, editing hosts, and blocked keyboard paths",
    );
}

#[test]
fn browser_input_planning_descriptors_track_text_controls_and_editing_hints() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form input-planning fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("form input-planning fixture should parse into browser document facts");

    assert_eq!(
        actual.input_planning_descriptors,
        case.expected.into_browser_document().input_planning_descriptors,
        "input-planning descriptors should preserve text-entry hints, datalist suggestions, validation blockers, form ownership, and editing metadata",
    );
}

#[test]
fn browser_drag_drop_descriptors_track_draggable_state_handlers_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive drag/drop fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive drag/drop fixture should parse into browser document facts");

    assert_eq!(
        actual.drag_drop_descriptors,
        case.expected.into_browser_document().drag_drop_descriptors,
        "drag/drop descriptors should preserve draggable state, drag/drop handlers, pointer handlers, and blocked drag paths",
    );
}

#[test]
fn browser_clipboard_interaction_descriptors_track_editing_handlers_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive clipboard fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive clipboard fixture should parse into browser document facts");

    assert_eq!(
        actual.clipboard_interaction_descriptors,
        case.expected
            .into_browser_document()
            .clipboard_interaction_descriptors,
        "clipboard-interaction descriptors should preserve copy/cut/paste handlers, input hooks, editing hosts, and blocked clipboard paths",
    );
}

#[test]
fn browser_selection_interaction_descriptors_track_editing_handlers_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive selection fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive selection fixture should parse into browser document facts");

    assert_eq!(
        actual.selection_interaction_descriptors,
        case.expected
            .into_browser_document()
            .selection_interaction_descriptors,
        "selection-interaction descriptors should preserve select handlers, selection-change hooks, editing hosts, and blocked selection paths",
    );
}

#[test]
fn browser_composition_interaction_descriptors_track_ime_and_input_hooks() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive composition fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive composition fixture should parse into browser document facts");

    assert_eq!(
        actual.composition_interaction_descriptors,
        case.expected
            .into_browser_document()
            .composition_interaction_descriptors,
        "composition-interaction descriptors should preserve beforeinput/input hooks, editing hosts, text controls, and blocked composition paths",
    );
}

#[test]
fn browser_pointer_interaction_descriptors_track_handlers_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive pointer fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive pointer fixture should parse into browser document facts");

    assert_eq!(
        actual.pointer_interaction_descriptors,
        case.expected
            .into_browser_document()
            .pointer_interaction_descriptors,
        "pointer-interaction descriptors should preserve click, mouse, touch, pointer, wheel, drag/drop routing, command/editing context, and blocked pointer paths",
    );
}

#[test]
fn browser_scroll_interaction_descriptors_track_scrollbars_handlers_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-range-descriptor-page")
        .expect("ARIA range scroll fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA range scroll fixture should parse into browser document facts");

    assert_eq!(
        actual.scroll_interaction_descriptors,
        case.expected
            .into_browser_document()
            .scroll_interaction_descriptors,
        "scroll-interaction descriptors should preserve ARIA scrollbar value state and blocked scroll paths",
    );
}

#[test]
fn browser_global_state_descriptor_metadata_tracks_non_form_global_states() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive global-state fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive global-state fixture should parse into browser document facts");

    assert_eq!(
        actual.global_state_descriptors,
        case.expected.into_browser_document().global_state_descriptors,
        "global-state descriptors should preserve inert, hidden, focus, editing, drag, spellcheck, translate, and accesskey metadata outside form-specific summaries",
    );
}

#[test]
fn browser_global_state_descriptors_track_attributes_focus_and_blockers() {
    let actual = parse_browser_document(
        r#"<body>
            <section id=panel hidden inert tabindex=-1 accesskey="p ?" autofocus>Panel</section>
            <div id=editor title="Draft" contenteditable spellcheck=false translate=no draggable=true>Copy</div>
        </body>"#,
    )
    .expect("global state descriptor fixture should parse");

    let panel = actual
        .global_state_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("panel"))
        .expect("panel descriptor should be present");
    assert_eq!(
        panel.global_attribute_names,
        vec!["hidden", "inert", "tabindex", "accesskey", "autofocus"]
    );
    assert_eq!(panel.global_attribute_count, 5);
    assert!(panel.focus_navigation_hint);
    assert!(panel.global_state_blocked);
    assert_eq!(panel.global_state_block_reasons, vec!["hidden", "inert"]);

    let editor = actual
        .global_state_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("editor"))
        .expect("editor descriptor should be present");
    assert_eq!(
        editor.global_attribute_names,
        vec![
            "title",
            "contenteditable",
            "draggable",
            "spellcheck",
            "translate",
        ]
    );
    assert_eq!(editor.global_attribute_count, 5);
    assert!(!editor.focus_navigation_hint);
    assert!(!editor.global_state_blocked);
    assert!(editor.global_state_block_reasons.is_empty());
}

#[test]
fn browser_accessible_description_metadata_tracks_describedby_text() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");

    let mut cases = suite.cases.into_iter();
    let form = cases
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form describedby fixture case should exist");
    let interactive = cases
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive describedby fixture case should exist");

    let actual_interactive = parse_browser_document(&interactive.input)
        .expect("interactive describedby fixture should parse into browser document facts");
    assert_eq!(
        actual_interactive.interactive_elements,
        interactive
            .expected
            .into_browser_document()
            .interactive_elements,
        "interactive summaries should resolve aria-describedby text into accessible descriptions",
    );

    let actual_form = parse_browser_document(&form.input)
        .expect("form describedby fixture should parse into browser document facts");
    assert_eq!(
        actual_form.forms,
        form.expected.into_browser_document().forms,
        "form controls should resolve aria-describedby text into accessible descriptions",
    );
}

#[test]
fn browser_aria_state_relation_metadata_tracks_composite_and_live_states() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive ARIA state relation fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive ARIA state relation fixture should parse into browser document facts");

    assert_eq!(
        actual.interactive_elements,
        case.expected.into_browser_document().interactive_elements,
        "interactive summaries should preserve ARIA relationship, popup, modal, validation, disabled, required, and live-region states",
    );
}

#[test]
fn browser_aria_relation_descriptors_track_targets_and_resolution_state() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-relation-descriptor-page")
        .expect("ARIA relation fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA relation fixture should parse into browser document facts");

    assert_eq!(
        actual.aria_relation_descriptors,
        case.expected.into_browser_document().aria_relation_descriptors,
        "ARIA relation descriptors should preserve relation attributes, target counts, resolved target text, and unresolved-id diagnostics",
    );
}

#[test]
fn browser_aria_relation_descriptors_track_unresolved_idrefs() {
    let actual = parse_browser_document(
        r#"<body>
            <div id=source aria-details="details missing" aria-errormessage=error aria-flowto="next absent">Source</div>
            <p id=details>Detailed help</p>
            <p id=error>Required value</p>
            <p id=next>Next step</p>
        </body>"#,
    )
    .expect("ARIA relation unresolved-id fixture should parse");

    let relation = actual
        .aria_relation_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("source"))
        .expect("source relation descriptor should be present");

    assert_eq!(
        relation.relation_attribute_names,
        vec!["aria-details", "aria-errormessage", "aria-flowto"]
    );
    assert_eq!(relation.relation_attribute_count, 3);
    assert_eq!(relation.relation_target_count, 5);
    assert_eq!(relation.details_text, vec!["Detailed help"]);
    assert_eq!(relation.errormessage_text, vec!["Required value"]);
    assert_eq!(relation.flowto_text, vec!["Next step"]);
    assert_eq!(
        relation.unresolved_relation_targets,
        vec!["missing", "absent"]
    );
    assert!(relation.relation_blocked);
    assert_eq!(relation.relation_block_reasons, vec!["unresolved-idref"]);
}

#[test]
fn browser_aria_name_descriptors_track_label_sources_and_resolution() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-name-descriptor-page")
        .expect("ARIA name fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA name fixture should parse into browser document facts");

    assert_eq!(
        actual.aria_name_descriptors,
        case.expected.into_browser_document().aria_name_descriptors,
        "ARIA name descriptors should preserve name sources, resolved label text, and unresolved label diagnostics",
    );
}

#[test]
fn browser_aria_name_descriptors_track_unresolved_labelledby_targets() {
    let actual = parse_browser_document(
        r#"<body>
            <button id=save aria-labelledby="save-label missing">Save</button>
            <span id=save-label>Save changes</span>
        </body>"#,
    )
    .expect("ARIA name unresolved-id fixture should parse");

    let name = actual
        .aria_name_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("save"))
        .expect("save button name descriptor should be present");

    assert_eq!(name.role, "control");
    assert_eq!(name.accessible_name.as_deref(), Some("Save changes"));
    assert_eq!(name.aria_labelledby, vec!["save-label", "missing"]);
    assert_eq!(name.labelledby_text, vec!["Save changes"]);
    assert_eq!(name.name_source, "aria-labelledby");
    assert_eq!(name.name_attribute_names, vec!["aria-labelledby"]);
    assert_eq!(name.name_attribute_count, 1);
    assert_eq!(name.label_target_count, 2);
    assert_eq!(name.unresolved_label_targets, vec!["missing"]);
    assert!(name.name_blocked);
    assert_eq!(name.name_block_reasons, vec!["unresolved-idref"]);
}

#[test]
fn browser_aria_description_descriptors_track_description_sources_and_resolution() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "aria-description-descriptor-page")
        .expect("ARIA description fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA description fixture should parse into browser document facts");

    assert_eq!(
        actual.aria_description_descriptors,
        case.expected
            .into_browser_document()
            .aria_description_descriptors,
        "ARIA description descriptors should preserve description sources, resolved help text, and unresolved idref diagnostics",
    );
}

#[test]
fn browser_aria_description_descriptors_track_unresolved_describedby_targets() {
    let actual = parse_browser_document(
        r#"<body>
            <button id=save aria-label="Save" aria-describedby="save-help missing">Save</button>
            <span id=save-help>Persists changes</span>
        </body>"#,
    )
    .expect("ARIA description unresolved-id fixture should parse");

    let description = actual
        .aria_description_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("save"))
        .expect("save button description descriptor should be present");

    assert_eq!(description.role, "control");
    assert_eq!(description.accessible_name.as_deref(), Some("Save"));
    assert_eq!(
        description.accessible_description.as_deref(),
        Some("Persists changes")
    );
    assert_eq!(description.aria_describedby, vec!["save-help", "missing"]);
    assert_eq!(description.describedby_text, vec!["Persists changes"]);
    assert_eq!(description.description_source, "aria-describedby");
    assert_eq!(
        description.description_attribute_names,
        vec!["aria-describedby"]
    );
    assert_eq!(description.description_attribute_count, 1);
    assert_eq!(description.description_target_count, 2);
    assert_eq!(description.unresolved_description_targets, vec!["missing"]);
    assert!(description.description_blocked);
    assert_eq!(
        description.description_block_reasons,
        vec!["unresolved-idref"]
    );
}

#[test]
fn browser_disclosure_descriptor_metadata_tracks_details_and_dialogs() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive fixture should parse into browser document facts");

    assert_eq!(
        actual.disclosures,
        case.expected.into_browser_document().disclosures,
        "disclosure summaries should preserve details summary text and dialog naming metadata",
    );
}

#[test]
fn browser_disclosure_state_descriptor_metadata_tracks_details_and_dialogs() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("interactive disclosure-state fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("interactive disclosure-state fixture should parse into browser document facts");

    assert_eq!(
        actual.disclosure_state_descriptors,
        case.expected.into_browser_document().disclosure_state_descriptors,
        "disclosure-state descriptors should preserve details/dialog open, summary, modal, naming, and grouping metadata",
    );
}

#[test]
fn browser_navigation_attribution_metadata_tracks_links_areas_and_form_targets() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");

    let legacy = suite
        .cases
        .into_iter()
        .find(|case| case.id == "legacy-directory-page")
        .expect("legacy directory fixture case should exist");
    let legacy_actual = parse_browser_document(&legacy.input)
        .expect("legacy directory fixture should parse into browser document facts");
    assert_eq!(
        legacy_actual.links,
        legacy.expected.into_browser_document().links,
        "link metadata should preserve attribution source registration URLs",
    );

    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let forms = suite
        .cases
        .into_iter()
        .find(|case| case.id == "form-accessibility-document-page")
        .expect("form accessibility fixture case should exist");
    let forms_actual = parse_browser_document(&forms.input)
        .expect("form accessibility fixture should parse into browser document facts");
    assert_eq!(
        forms_actual.forms,
        forms.expected.into_browser_document().forms,
        "form metadata should preserve rel tokens and base-target effective submitter targets",
    );
}

#[test]
fn browser_component_hydration_metadata_tracks_custom_elements_slots_parts_and_data() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "component-template-page")
        .expect("component template fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("component template fixture should parse into browser document facts");

    assert_eq!(
        actual.component_hydration_targets,
        case.expected
            .into_browser_document()
            .component_hydration_targets,
        "component hydration metadata should preserve custom elements, slots, parts, data attributes, and canvas fallback text",
    );
}

#[test]
fn browser_component_hydration_descriptors_track_kinds_signals_and_blockers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "component-template-page")
        .expect("component template fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("component template fixture should parse into browser document facts");

    assert_eq!(
        actual.component_hydration_descriptors,
        case.expected
            .into_browser_document()
            .component_hydration_descriptors,
        "component hydration descriptors should preserve target kind, component, slot, part, data, canvas, and blocker metadata",
    );
}

#[test]
fn browser_component_hydration_descriptors_track_invalid_targets() {
    let actual = parse_browser_document(
        "<body><template shadowrootmode=light><span>Bad mode</span></template>\
         <slot name=\"   \">Fallback</slot><span slot=\"   \">Item</span>\
         <canvas id=empty></canvas><button is=bad>Bad</button>",
    )
    .expect("invalid component hydration fixture should parse into browser document facts");

    let blocked: Vec<_> = actual
        .component_hydration_descriptors
        .iter()
        .filter(|descriptor| descriptor.hydration_blocked)
        .collect();

    assert_eq!(blocked.len(), 5);
    assert_eq!(
        blocked[0].hydration_block_reasons,
        vec!["invalid-shadowrootmode"]
    );
    assert_eq!(blocked[1].hydration_block_reasons, vec!["blank-slot-name"]);
    assert_eq!(
        blocked[2].hydration_block_reasons,
        vec!["blank-slot-assignment"]
    );
    assert_eq!(
        blocked[3].hydration_block_reasons,
        vec!["missing-canvas-fallback-text"]
    );
    assert_eq!(
        blocked[4].hydration_block_reasons,
        vec!["invalid-custom-element-name"]
    );
}

#[test]
fn browser_template_descriptors_track_shadowroot_mode_and_flags() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "component-template-page")
        .expect("component template fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("component template fixture should parse into browser document facts");

    assert_eq!(
        actual.template_descriptors,
        case.expected.into_browser_document().template_descriptors,
        "template descriptors should preserve declarative shadow root mode, flags, and content summary",
    );
}

#[test]
fn browser_template_descriptors_track_invalid_shadowroot_modes_and_orphan_flags() {
    let actual = parse_browser_document(
        r#"<body>
            <template id=bad shadowrootmode=sideways shadowrootdelegatesfocus>Bad mode</template>
            <template id=flags shadowrootserializable>Plain template</template>
        </body>"#,
    )
    .expect("template blocker fixture should parse");

    let bad = actual
        .template_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("bad"))
        .expect("bad template descriptor should be present");

    assert_eq!(bad.template_kind, "declarative-shadow-root");
    assert_eq!(bad.shadowrootmode.as_deref(), Some("sideways"));
    assert_eq!(
        bad.shadowroot_attribute_names,
        vec!["shadowrootmode", "shadowrootdelegatesfocus"]
    );
    assert_eq!(bad.shadowroot_attribute_count, 2);
    assert!(bad.declarative_shadow_root);
    assert!(!bad.shadowroot_mode_valid);
    assert!(bad.template_blocked);
    assert_eq!(bad.template_block_reasons, vec!["invalid-shadowrootmode"]);

    let flags = actual
        .template_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("flags"))
        .expect("flags template descriptor should be present");

    assert_eq!(flags.template_kind, "inert-template");
    assert_eq!(
        flags.shadowroot_attribute_names,
        vec!["shadowrootserializable"]
    );
    assert!(!flags.declarative_shadow_root);
    assert!(!flags.shadowroot_mode_valid);
    assert!(flags.template_blocked);
    assert_eq!(
        flags.template_block_reasons,
        vec!["shadowroot-flags-without-mode"]
    );
}

#[test]
fn browser_slot_descriptors_track_slot_outlets_assignments_and_fallbacks() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "component-template-page")
        .expect("component template fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("component template fixture should parse into browser document facts");

    assert_eq!(
        actual.slot_descriptors,
        case.expected.into_browser_document().slot_descriptors,
        "slot descriptors should distinguish slot outlets, slotted elements, named/default slots, parts, and fallback text",
    );
}

#[test]
fn browser_slot_descriptors_track_blank_slot_blockers() {
    let actual = parse_browser_document(
        r#"<body>
            <template shadowrootmode=open>
                <slot name="   ">Blank fallback</slot>
            </template>
            <x-card><span slot="   ">Blank assignment</span><span slot="">Default assignment</span></x-card>
        </body>"#,
    )
    .expect("slot blocker fixture should parse");

    let blank_slot = actual
        .slot_descriptors
        .iter()
        .find(|descriptor| descriptor.slot_name.as_deref() == Some("   "))
        .expect("blank named slot descriptor should be present");

    assert_eq!(blank_slot.slot_kind, "slot-element");
    assert!(blank_slot.named_slot);
    assert!(!blank_slot.default_slot);
    assert_eq!(blank_slot.fallback_text, "Blank fallback");
    assert_eq!(blank_slot.fallback_word_count, 2);
    assert!(blank_slot.slot_blocked);
    assert_eq!(blank_slot.slot_block_reasons, vec!["blank-slot-name"]);

    let blank_assignment = actual
        .slot_descriptors
        .iter()
        .find(|descriptor| descriptor.slot.as_deref() == Some("   "))
        .expect("blank slot assignment descriptor should be present");

    assert_eq!(blank_assignment.slot_kind, "slotted-element");
    assert!(blank_assignment.named_slot);
    assert!(!blank_assignment.default_slot);
    assert!(blank_assignment.slot_blocked);
    assert_eq!(
        blank_assignment.slot_block_reasons,
        vec!["blank-slot-assignment"]
    );

    let default_assignment = actual
        .slot_descriptors
        .iter()
        .find(|descriptor| descriptor.slot.as_deref() == Some(""))
        .expect("default slot assignment descriptor should be present");

    assert_eq!(default_assignment.slot_kind, "slotted-element");
    assert!(default_assignment.default_slot);
    assert!(!default_assignment.named_slot);
    assert!(!default_assignment.slot_blocked);
}

#[test]
fn browser_custom_element_descriptors_track_autonomous_and_customized_builtins() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "component-template-page")
        .expect("component template fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("component template fixture should parse into browser document facts");

    assert_eq!(
        actual.custom_element_descriptors,
        case
            .expected
            .into_browser_document()
            .custom_element_descriptors,
        "custom element descriptors should preserve autonomous and customized built-in upgrade hints",
    );
}

#[test]
fn browser_custom_element_descriptors_track_invalid_definition_hints() {
    let actual = parse_browser_document(
        r#"<body>
            <button is=plain-button>Valid built-in</button>
            <button is=plain>Invalid built-in</button>
            <button is="">Empty built-in</button>
            <x-card is=fancy-button>Autonomous with is</x-card>
        </body>"#,
    )
    .expect("custom element blocker fixture should parse");

    let valid = actual
        .custom_element_descriptors
        .iter()
        .find(|descriptor| descriptor.custom_element_is.as_deref() == Some("plain-button"))
        .expect("valid customized built-in descriptor should be present");

    assert_eq!(valid.custom_element_kind, "customized-built-in");
    assert_eq!(valid.definition_name.as_deref(), Some("plain-button"));
    assert!(valid.custom_element_name_valid);
    assert!(valid.customized_builtin);
    assert_eq!(valid.extends_element.as_deref(), Some("button"));
    assert!(!valid.custom_element_blocked);

    let invalid = actual
        .custom_element_descriptors
        .iter()
        .find(|descriptor| descriptor.custom_element_is.as_deref() == Some("plain"))
        .expect("invalid customized built-in descriptor should be present");

    assert_eq!(invalid.custom_element_kind, "customized-built-in");
    assert!(!invalid.custom_element_name_valid);
    assert!(invalid.custom_element_blocked);
    assert_eq!(
        invalid.custom_element_block_reasons,
        vec!["invalid-custom-element-name"]
    );

    let empty = actual
        .custom_element_descriptors
        .iter()
        .find(|descriptor| descriptor.custom_element_is.as_deref() == Some(""))
        .expect("empty customized built-in descriptor should be present");

    assert_eq!(empty.custom_element_kind, "customized-built-in");
    assert!(!empty.custom_element_name_valid);
    assert!(empty.custom_element_blocked);
    assert_eq!(empty.custom_element_block_reasons, vec!["empty-is-value"]);

    let autonomous_with_is = actual
        .custom_element_descriptors
        .iter()
        .find(|descriptor| descriptor.element == "x-card")
        .expect("autonomous-with-is descriptor should be present");

    assert_eq!(autonomous_with_is.custom_element_kind, "autonomous-with-is");
    assert!(autonomous_with_is.autonomous_custom_element);
    assert!(!autonomous_with_is.customized_builtin);
    assert!(autonomous_with_is.custom_element_blocked);
    assert_eq!(
        autonomous_with_is.custom_element_block_reasons,
        vec!["is-on-autonomous-custom-element"]
    );
}

#[test]
fn browser_canvas_descriptors_track_dimensions_fallback_and_handlers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "component-template-page")
        .expect("component template fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("component template fixture should parse into browser document facts");

    assert_eq!(
        actual.canvas_descriptors,
        case.expected.into_browser_document().canvas_descriptors,
        "canvas descriptors should preserve dimensions, fallback text, part/data context, and handlers",
    );
}

#[test]
fn browser_canvas_descriptors_track_missing_size_and_fallback_blockers() {
    let actual = parse_browser_document(
        r#"<body>
            <canvas id=paint class="surface primary" width=640 data-renderer=webgl part=viewport
                onpointerdown=startPaint() onkeydown=hotkey() onload=ready()>Fallback drawing</canvas>
            <canvas id=empty></canvas>
        </body>"#,
    )
    .expect("canvas descriptor fixture should parse");

    let paint = actual
        .canvas_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("paint"))
        .expect("paint canvas descriptor should be present");

    assert_eq!(paint.classes, vec!["surface", "primary"]);
    assert_eq!(paint.width.as_deref(), Some("640"));
    assert_eq!(paint.height, None);
    assert!(paint.has_width);
    assert!(!paint.has_height);
    assert_eq!(paint.fallback_text, "Fallback drawing");
    assert_eq!(paint.fallback_word_count, 2);
    assert_eq!(paint.part, vec!["viewport"]);
    assert_eq!(paint.data_attribute_names, vec!["data-renderer"]);
    assert_eq!(
        paint.event_handlers,
        vec!["onpointerdown", "onkeydown", "onload"]
    );
    assert_eq!(paint.pointer_handlers, vec!["onpointerdown"]);
    assert_eq!(paint.keyboard_handlers, vec!["onkeydown"]);
    assert_eq!(paint.lifecycle_handlers, vec!["onload"]);
    assert!(paint.canvas_blocked);
    assert_eq!(paint.canvas_block_reasons, vec!["missing-height"]);

    let empty = actual
        .canvas_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("empty"))
        .expect("empty canvas descriptor should be present");

    assert!(!empty.has_width);
    assert!(!empty.has_height);
    assert!(empty.fallback_text.is_empty());
    assert!(empty.canvas_blocked);
    assert_eq!(
        empty.canvas_block_reasons,
        vec!["missing-width", "missing-height", "missing-fallback-text"]
    );
}

#[test]
fn browser_data_attribute_descriptor_metadata_tracks_custom_and_standard_elements() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "component-template-page")
        .expect("component template fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("component template fixture should parse into browser document facts");

    assert_eq!(
        actual.data_attribute_descriptors,
        case.expected
            .into_browser_document()
            .data_attribute_descriptors,
        "data attribute descriptors should preserve custom element, slot, part, and visible-text context for all data-* carriers",
    );
}

#[test]
fn browser_data_attribute_descriptors_track_names_counts_and_json_hints() {
    let actual = parse_browser_document(
        r#"<body>
            <div id=card data-controller=dashboard data-options='{"compact":true}' data-empty>Alpha</div>
            <x-chart data-series='[1,2]' data-state=ready></x-chart>
        </body>"#,
    )
    .expect("data attribute descriptor fixture should parse");

    let card = actual
        .data_attribute_descriptors
        .iter()
        .find(|descriptor| descriptor.id.as_deref() == Some("card"))
        .expect("card descriptor should be present");
    assert_eq!(
        card.data_attribute_names,
        vec!["data-controller", "data-options", "data-empty"]
    );
    assert_eq!(card.data_attribute_count, 3);
    assert_eq!(card.json_data_attribute_names, vec!["data-options"]);

    let chart = actual
        .data_attribute_descriptors
        .iter()
        .find(|descriptor| descriptor.element == "x-chart")
        .expect("custom chart descriptor should be present");
    assert_eq!(
        chart.data_attribute_names,
        vec!["data-series", "data-state"]
    );
    assert_eq!(chart.data_attribute_count, 2);
    assert_eq!(chart.json_data_attribute_names, vec!["data-series"]);
}

#[test]
fn browser_event_handler_descriptors_track_document_and_element_wiring() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "event-handler-page")
        .expect("event handler fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("event handler fixture should parse into browser document facts");

    assert_eq!(
        actual.event_handler_descriptors,
        case.expected
            .into_browser_document()
            .event_handler_descriptors,
        "event handler descriptors should preserve document/body handlers and categorize inline element handlers without evaluating script text",
    );
}

#[test]
fn browser_lifecycle_event_descriptors_track_load_and_error_recovery_hooks() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "event-handler-page")
        .expect("event handler fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("event handler fixture should parse into browser document facts");

    assert_eq!(
        actual.lifecycle_event_descriptors,
        case.expected.into_browser_document().lifecycle_event_descriptors,
        "lifecycle-event descriptors should preserve document/body load hooks and element error-recovery handlers",
    );
}

#[test]
fn browser_animation_interaction_descriptors_track_css_timeline_hooks() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "event-handler-page")
        .expect("event handler fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("event handler fixture should parse into browser document facts");

    assert_eq!(
        actual.animation_interaction_descriptors,
        case.expected
            .into_browser_document()
            .animation_interaction_descriptors,
        "animation-interaction descriptors should preserve CSS animation and transition inline hooks",
    );
}

#[test]
fn browser_fullscreen_interaction_descriptors_track_embedded_policy_hints() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "embedded-resource-page")
        .expect("embedded context fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("embedded context fixture should parse into browser document facts");

    assert_eq!(
        actual.fullscreen_interaction_descriptors,
        case.expected
            .into_browser_document()
            .fullscreen_interaction_descriptors,
        "fullscreen-interaction descriptors should preserve iframe fullscreen policy hints",
    );
}

#[test]
fn browser_context_menu_interaction_descriptors_track_menu_invokers_and_handlers() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");
    let case = suite
        .cases
        .into_iter()
        .find(|case| case.id == "interactive-element-state-page")
        .expect("ARIA interaction fixture case should exist");

    let actual = parse_browser_document(&case.input)
        .expect("ARIA interaction fixture should parse into browser document facts");

    assert_eq!(
        actual.context_menu_interaction_descriptors,
        case.expected
            .into_browser_document()
            .context_menu_interaction_descriptors,
        "context-menu descriptors should preserve ARIA menu invokers and contextmenu hooks",
    );
}

impl ExpectedBrowserDocument {
    fn into_browser_document(self) -> BrowserDocument {
        let metadata = self.metadata.into_browser_document_metadata();
        let resources: Vec<_> = self
            .resources
            .into_iter()
            .map(ExpectedResource::into_browser_resource)
            .collect();
        let anchors: Vec<_> = self
            .anchors
            .into_iter()
            .map(ExpectedAnchor::into_browser_anchor)
            .collect();
        let anchor_descriptors = self
            .anchor_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedAnchorDescriptor::into_browser_anchor_descriptor)
            .collect();
        let headings: Vec<_> = self
            .headings
            .into_iter()
            .map(ExpectedHeading::into_browser_heading)
            .collect();
        let heading_descriptors = self
            .heading_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedHeadingDescriptor::into_browser_heading_descriptor)
            .collect();
        let text_semantics: Vec<_> = self
            .text_semantics
            .into_iter()
            .map(ExpectedTextSemantic::into_browser_text_semantic)
            .collect();
        let text_semantic_descriptors = self
            .text_semantic_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedTextSemanticDescriptor::into_browser_text_semantic_descriptor)
            .collect();
        let text_flow_descriptors = self
            .text_flow_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedTextFlowDescriptor::into_browser_text_flow_descriptor)
            .collect();
        let navigation_group_descriptors = self
            .navigation_group_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedNavigationGroupDescriptor::into_browser_navigation_group_descriptor)
            .collect();
        let section_landmark_descriptors = self
            .section_landmark_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedSectionLandmarkDescriptor::into_browser_section_landmark_descriptor)
            .collect();
        let scripts: Vec<_> = self
            .scripts
            .into_iter()
            .map(ExpectedScript::into_browser_script)
            .collect();
        let images: Vec<_> = self
            .images
            .into_iter()
            .map(ExpectedImage::into_browser_image)
            .collect();
        let stylesheets: Vec<_> = self
            .stylesheets
            .into_iter()
            .map(ExpectedStylesheet::into_browser_stylesheet)
            .collect();
        let media: Vec<_> = self
            .media
            .into_iter()
            .map(ExpectedMedia::into_browser_media)
            .collect();
        let embedded_contexts: Vec<_> = self
            .embedded_contexts
            .into_iter()
            .map(ExpectedEmbeddedContext::into_browser_embedded_context)
            .collect();
        let event_handler_descriptors: Vec<_> = self
            .event_handler_descriptors
            .into_iter()
            .map(ExpectedEventHandlerDescriptor::into_browser_event_handler_descriptor)
            .collect();
        let lifecycle_event_descriptors = self
            .lifecycle_event_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedLifecycleEventDescriptor::into_browser_lifecycle_event_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_lifecycle_event_descriptors(&event_handler_descriptors));
        let animation_interaction_descriptors = self
            .animation_interaction_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedAnimationInteractionDescriptor::into_browser_animation_interaction_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_animation_interaction_descriptors(&event_handler_descriptors)
            });
        let global_state_descriptors: Vec<_> = self
            .global_state_descriptors
            .into_iter()
            .map(ExpectedGlobalStateDescriptor::into_browser_global_state_descriptor)
            .collect();
        let resource_endpoint_descriptors = self
            .resource_endpoint_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedResourceEndpointDescriptor::into_browser_resource_endpoint_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_resource_endpoint_descriptors(&metadata, &resources));
        let link_resource_descriptors = self
            .link_resource_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedLinkResourceDescriptor::into_browser_link_resource_descriptor)
            .collect();
        let media_playback_descriptors = self
            .media_playback_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedMediaPlaybackDescriptor::into_browser_media_playback_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_media_playback_descriptors(&media));
        let media_resource_descriptors = self
            .media_resource_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedMediaResourceDescriptor::into_browser_media_resource_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_media_resource_descriptors(&media));
        let embedded_policy_descriptors = self
            .embedded_policy_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedEmbeddedPolicyDescriptor::into_browser_embedded_policy_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_embedded_policy_descriptors(&embedded_contexts));
        let fullscreen_interaction_descriptors = self
            .fullscreen_interaction_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedFullscreenInteractionDescriptor::into_browser_fullscreen_interaction_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_fullscreen_interaction_descriptors(
                    &embedded_policy_descriptors,
                    &event_handler_descriptors,
                )
            });
        let script_execution_descriptors = self
            .script_execution_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedScriptExecutionDescriptor::into_browser_script_execution_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_script_execution_descriptors(&scripts));
        let script_storage_access_descriptors = self
            .script_storage_access_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedScriptStorageAccessDescriptor::into_browser_script_storage_access_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_script_storage_access_descriptors(&scripts));
        let script_worker_messaging_descriptors = self
            .script_worker_messaging_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedScriptWorkerMessagingDescriptor::into_browser_script_worker_messaging_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_script_worker_messaging_descriptors(&scripts));
        let script_module_graph_descriptors = self
            .script_module_graph_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedScriptModuleGraphDescriptor::into_browser_script_module_graph_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_script_module_graph_descriptors(&scripts, &resources));
        let stylesheet_planning_descriptors = self
            .stylesheet_planning_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedStylesheetPlanningDescriptor::into_browser_stylesheet_planning_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_stylesheet_planning_descriptors(&stylesheets));
        let image_candidate_descriptors = self
            .image_candidate_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedImageCandidateDescriptor::into_browser_image_candidate_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_image_candidate_descriptors(&images));
        let disclosures: Vec<_> = self
            .disclosures
            .into_iter()
            .map(ExpectedDisclosure::into_browser_disclosure)
            .collect();
        let command_elements: Vec<_> = self
            .command_elements
            .into_iter()
            .map(ExpectedCommandElement::into_browser_command_element)
            .collect();
        let popovers: Vec<_> = self
            .popovers
            .into_iter()
            .map(ExpectedPopover::into_browser_popover)
            .collect();
        let popover_descriptors = self
            .popover_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedPopoverDescriptor::into_browser_popover_descriptor)
            .collect();
        let aria_collection_descriptors = self
            .aria_collection_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedAriaCollectionDescriptor::into_browser_aria_collection_descriptor)
            .collect();
        let aria_ranges: Vec<_> = self
            .aria_ranges
            .into_iter()
            .map(ExpectedAriaRange::into_browser_aria_range)
            .collect();
        let interactive_elements: Vec<_> = self
            .interactive_elements
            .into_iter()
            .map(ExpectedInteractiveElement::into_browser_interactive_element)
            .collect();
        let forms: Vec<_> = self
            .forms
            .into_iter()
            .map(ExpectedForm::into_browser_form)
            .collect();
        let form_control_descriptors = self
            .form_control_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedFormControlDescriptor::into_browser_form_control_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_form_control_descriptors(&forms));
        let form_association_descriptors = self
            .form_association_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedFormAssociationDescriptor::into_browser_form_association_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_form_association_descriptors(&forms));
        let form_autofill_descriptors = self
            .form_autofill_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedFormAutofillDescriptor::into_browser_form_autofill_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_form_autofill_descriptors(&forms));
        let form_submission_descriptors = self
            .form_submission_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedFormSubmissionDescriptor::into_browser_form_submission_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_form_submission_descriptors(&forms));
        let form_reset_descriptors = self
            .form_reset_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedFormResetDescriptor::into_browser_form_reset_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_form_reset_descriptors(&forms));
        let form_validation_descriptors = self
            .form_validation_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedFormValidationDescriptor::into_browser_form_validation_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_form_validation_descriptors(&forms));
        let context_menu_interaction_descriptors = self
            .context_menu_interaction_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedContextMenuInteractionDescriptor::into_browser_context_menu_interaction_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_context_menu_interaction_descriptors(
                    &interactive_elements,
                    &event_handler_descriptors,
                )
            });
        let disclosure_state_descriptors = self
            .disclosure_state_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedDisclosureStateDescriptor::into_browser_disclosure_state_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_disclosure_state_descriptors(&disclosures));
        let activation_descriptors = self
            .activation_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedActivationDescriptor::into_browser_activation_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_activation_descriptors(&command_elements, &popovers, &disclosures)
            });
        let focus_navigation_descriptors = self
            .focus_navigation_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedFocusNavigationDescriptor::into_browser_focus_navigation_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_focus_navigation_descriptors(&interactive_elements));
        let keyboard_interaction_descriptors = self
            .keyboard_interaction_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedKeyboardInteractionDescriptor::into_browser_keyboard_interaction_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| expected_keyboard_interaction_descriptors(&interactive_elements));
        let input_planning_descriptors = self
            .input_planning_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedInputPlanningDescriptor::into_browser_input_planning_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| expected_input_planning_descriptors(&forms, &interactive_elements));
        let drag_drop_descriptors = self
            .drag_drop_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedDragDropDescriptor::into_browser_drag_drop_descriptor)
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_drag_drop_descriptors(
                    &interactive_elements,
                    &global_state_descriptors,
                    &event_handler_descriptors,
                )
            });
        let clipboard_interaction_descriptors = self
            .clipboard_interaction_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedClipboardInteractionDescriptor::into_browser_clipboard_interaction_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_clipboard_interaction_descriptors(
                    &forms,
                    &interactive_elements,
                    &event_handler_descriptors,
                )
            });
        let selection_interaction_descriptors = self
            .selection_interaction_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedSelectionInteractionDescriptor::into_browser_selection_interaction_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_selection_interaction_descriptors(
                    &forms,
                    &interactive_elements,
                    &event_handler_descriptors,
                )
            });
        let composition_interaction_descriptors = self
            .composition_interaction_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedCompositionInteractionDescriptor::into_browser_composition_interaction_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_composition_interaction_descriptors(
                    &forms,
                    &interactive_elements,
                    &event_handler_descriptors,
                )
            });
        let pointer_interaction_descriptors = self
            .pointer_interaction_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedPointerInteractionDescriptor::into_browser_pointer_interaction_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_pointer_interaction_descriptors(
                    &interactive_elements,
                    &event_handler_descriptors,
                )
            });
        let scroll_interaction_descriptors = self
            .scroll_interaction_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(
                        ExpectedScrollInteractionDescriptor::into_browser_scroll_interaction_descriptor,
                    )
                    .collect()
            })
            .unwrap_or_else(|| {
                expected_scroll_interaction_descriptors(
                    &aria_ranges,
                    &interactive_elements,
                    &event_handler_descriptors,
                )
            });
        let canvas_descriptors = self
            .canvas_descriptors
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(ExpectedCanvasDescriptor::into_browser_canvas_descriptor)
                    .collect()
            })
            .unwrap_or_default();
        let component_hydration_descriptors = self
            .component_hydration_descriptors
            .unwrap_or_default()
            .into_iter()
            .map(ExpectedComponentHydrationDescriptor::into_browser_component_hydration_descriptor)
            .collect();

        BrowserDocument {
            title: self.title,
            base_href: self.base_href,
            base_target: self.base_target,
            metadata,
            document_lang: self.document_lang,
            document_dir: self.document_dir,
            body_id: self.body_id,
            body_classes: self.body_classes,
            body_lang: self.body_lang,
            body_dir: self.body_dir,
            document_event_handlers: self.document_event_handlers,
            body_event_handlers: self.body_event_handlers,
            event_handler_descriptors,
            lifecycle_event_descriptors,
            animation_interaction_descriptors,
            fullscreen_interaction_descriptors,
            context_menu_interaction_descriptors,
            body_text: self.body_text,
            metas: self
                .metas
                .into_iter()
                .map(ExpectedMeta::into_browser_meta)
                .collect(),
            resources,
            scripts,
            script_execution_descriptors,
            script_storage_access_descriptors,
            script_worker_messaging_descriptors,
            script_module_graph_descriptors,
            stylesheets,
            stylesheet_planning_descriptors,
            document_policy_descriptors: self
                .document_policy_descriptors
                .into_iter()
                .map(ExpectedDocumentPolicyDescriptor::into_browser_document_policy_descriptor)
                .collect(),
            loading_hint_descriptors: self
                .loading_hint_descriptors
                .into_iter()
                .map(ExpectedLoadingHintDescriptor::into_browser_loading_hint_descriptor)
                .collect(),
            fetch_policy_descriptors: self
                .fetch_policy_descriptors
                .into_iter()
                .map(ExpectedFetchPolicyDescriptor::into_browser_fetch_policy_descriptor)
                .collect(),
            resource_endpoint_descriptors,
            link_resource_descriptors,
            form_policy_descriptors: self
                .form_policy_descriptors
                .into_iter()
                .map(ExpectedFormPolicyDescriptor::into_browser_form_policy_descriptor)
                .collect(),
            form_control_descriptors,
            form_association_descriptors,
            form_autofill_descriptors,
            form_submission_descriptors,
            form_reset_descriptors,
            form_validation_descriptors,
            anchors,
            anchor_descriptors,
            headings,
            heading_descriptors,
            text_semantics,
            text_semantic_descriptors,
            text_flow_descriptors,
            navigation_target_descriptors: self
                .navigation_target_descriptors
                .into_iter()
                .map(ExpectedNavigationTargetDescriptor::into_browser_navigation_target_descriptor)
                .collect(),
            navigation_groups: self
                .navigation_groups
                .into_iter()
                .map(ExpectedNavigationGroup::into_browser_navigation_group)
                .collect(),
            navigation_group_descriptors,
            section_landmarks: self
                .section_landmarks
                .into_iter()
                .map(ExpectedSectionLandmark::into_browser_section_landmark)
                .collect(),
            section_landmark_descriptors,
            command_elements,
            activation_descriptors,
            popovers,
            popover_descriptors,
            aria_collections: self
                .aria_collections
                .into_iter()
                .map(ExpectedAriaCollection::into_browser_aria_collection)
                .collect(),
            aria_collection_descriptors,
            aria_ranges,
            aria_live_regions: self
                .aria_live_regions
                .into_iter()
                .map(ExpectedAriaLiveRegion::into_browser_aria_live_region)
                .collect(),
            aria_name_descriptors: self
                .aria_name_descriptors
                .unwrap_or_default()
                .into_iter()
                .map(ExpectedAriaNameDescriptor::into_browser_aria_name_descriptor)
                .collect(),
            aria_description_descriptors: self
                .aria_description_descriptors
                .unwrap_or_default()
                .into_iter()
                .map(ExpectedAriaDescriptionDescriptor::into_browser_aria_description_descriptor)
                .collect(),
            aria_relation_descriptors: self
                .aria_relation_descriptors
                .into_iter()
                .map(ExpectedAriaRelationDescriptor::into_browser_aria_relation_descriptor)
                .collect(),
            links: self
                .links
                .into_iter()
                .map(ExpectedLink::into_browser_link)
                .collect(),
            images,
            image_candidate_descriptors,
            image_maps: self
                .image_maps
                .into_iter()
                .map(ExpectedImageMap::into_browser_image_map)
                .collect(),
            image_map_descriptors: self
                .image_map_descriptors
                .unwrap_or_default()
                .into_iter()
                .map(ExpectedImageMapDescriptor::into_browser_image_map_descriptor)
                .collect(),
            media,
            media_playback_descriptors,
            media_resource_descriptors,
            embedded_contexts,
            embedded_policy_descriptors,
            interactive_elements,
            focus_navigation_descriptors,
            keyboard_interaction_descriptors,
            input_planning_descriptors,
            drag_drop_descriptors,
            clipboard_interaction_descriptors,
            selection_interaction_descriptors,
            composition_interaction_descriptors,
            pointer_interaction_descriptors,
            scroll_interaction_descriptors,
            disclosures,
            disclosure_state_descriptors,
            template_descriptors: self
                .template_descriptors
                .unwrap_or_default()
                .into_iter()
                .map(ExpectedTemplateDescriptor::into_browser_template_descriptor)
                .collect(),
            slot_descriptors: self
                .slot_descriptors
                .unwrap_or_default()
                .into_iter()
                .map(ExpectedSlotDescriptor::into_browser_slot_descriptor)
                .collect(),
            custom_element_descriptors: self
                .custom_element_descriptors
                .unwrap_or_default()
                .into_iter()
                .map(ExpectedCustomElementDescriptor::into_browser_custom_element_descriptor)
                .collect(),
            canvas_descriptors,
            component_hydration_targets: self
                .component_hydration_targets
                .into_iter()
                .map(ExpectedComponentHydrationTarget::into_browser_component_hydration_target)
                .collect(),
            component_hydration_descriptors,
            data_attribute_descriptors: self
                .data_attribute_descriptors
                .into_iter()
                .map(ExpectedDataAttributeDescriptor::into_browser_data_attribute_descriptor)
                .collect(),
            global_state_descriptors,
            structured_data_descriptors: self
                .structured_data_descriptors
                .unwrap_or_default()
                .into_iter()
                .map(ExpectedStructuredDataDescriptor::into_browser_structured_data_descriptor)
                .collect(),
            structured_items: self
                .structured_items
                .into_iter()
                .map(ExpectedStructuredItem::into_browser_structured_item)
                .collect(),
            templates: self
                .templates
                .into_iter()
                .map(ExpectedTemplate::into_browser_template)
                .collect(),
            forms,
            tables: self
                .tables
                .into_iter()
                .map(ExpectedTable::into_browser_table)
                .collect(),
            table_cells: self
                .table_cells
                .into_iter()
                .map(ExpectedTableCell::into_browser_table_cell)
                .collect(),
            table_structure_descriptors: self
                .table_structure_descriptors
                .unwrap_or_default()
                .into_iter()
                .map(ExpectedTableStructureDescriptor::into_browser_table_structure_descriptor)
                .collect(),
        }
    }
}

impl ExpectedDocumentMetadata {
    fn into_browser_document_metadata(self) -> BrowserDocumentMetadata {
        BrowserDocumentMetadata {
            charset: self.charset,
            viewport: self.viewport,
            viewport_directives: self
                .viewport_directives
                .into_iter()
                .map(ExpectedMetadataDirective::into_browser_metadata_directive)
                .collect(),
            description: self.description,
            application_name: self.application_name,
            referrer_policy: self.referrer_policy,
            robots: self.robots,
            robots_directives: self.robots_directives,
            color_scheme: self.color_scheme,
            http_equiv_hints: self
                .http_equiv_hints
                .into_iter()
                .map(ExpectedHttpEquivHint::into_browser_http_equiv_hint)
                .collect(),
            resource_hints: self
                .resource_hints
                .into_iter()
                .map(ExpectedResourceHint::into_browser_resource_hint)
                .collect(),
            theme_colors: self
                .theme_colors
                .into_iter()
                .map(ExpectedThemeColor::into_browser_theme_color)
                .collect(),
            refresh: self.refresh.map(ExpectedRefresh::into_browser_refresh),
            canonical_url: self.canonical_url,
            resolved_canonical_url: self.resolved_canonical_url,
            manifest_url: self.manifest_url,
            resolved_manifest_url: self.resolved_manifest_url,
        }
    }
}

impl ExpectedMetadataDirective {
    fn into_browser_metadata_directive(self) -> BrowserMetadataDirective {
        BrowserMetadataDirective {
            name: self.name,
            value: self.value,
        }
    }
}

impl ExpectedHttpEquivHint {
    fn into_browser_http_equiv_hint(self) -> BrowserHttpEquivHint {
        BrowserHttpEquivHint {
            name: self.name,
            content: self.content,
        }
    }
}

impl ExpectedResourceHint {
    fn into_browser_resource_hint(self) -> BrowserResourceHint {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserResourceHint {
            kind: self.kind,
            url: self.url,
            resolved_url: self.resolved_url,
            rel: self.rel,
            rel_tokens,
            as_hint: self.as_hint,
            type_hint: self.type_hint,
            media: self.media,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            imagesrcset: self.imagesrcset,
            resolved_imagesrcset: self.resolved_imagesrcset,
            imagesizes: self.imagesizes,
        }
    }
}

impl ExpectedThemeColor {
    fn into_browser_theme_color(self) -> BrowserThemeColor {
        BrowserThemeColor {
            color: self.color,
            media: self.media,
        }
    }
}

impl ExpectedRefresh {
    fn into_browser_refresh(self) -> BrowserRefresh {
        BrowserRefresh {
            delay: self.delay,
            url: self.url,
            resolved_url: self.resolved_url,
        }
    }
}

impl ExpectedMeta {
    fn into_browser_meta(self) -> BrowserMeta {
        BrowserMeta {
            name: self.name,
            http_equiv: self.http_equiv,
            property: self.property,
            charset: self.charset,
            content: self.content,
        }
    }
}

impl ExpectedStructuredItem {
    fn into_browser_structured_item(self) -> BrowserStructuredItem {
        BrowserStructuredItem {
            id: self.id,
            item_type: self.item_type,
            item_id: self.item_id,
            resolved_item_id: self.resolved_item_id,
            item_ref: self.item_ref,
            properties: self
                .properties
                .into_iter()
                .map(ExpectedStructuredProperty::into_browser_structured_property)
                .collect(),
        }
    }
}

impl ExpectedStructuredProperty {
    fn into_browser_structured_property(self) -> BrowserStructuredProperty {
        BrowserStructuredProperty {
            name: self.name,
            value: self.value,
            value_url: self.value_url,
            resolved_value_url: self.resolved_value_url,
        }
    }
}

impl ExpectedStructuredDataDescriptor {
    fn into_browser_structured_data_descriptor(self) -> BrowserStructuredDataDescriptor {
        BrowserStructuredDataDescriptor {
            id: self.id,
            item_type: self.item_type,
            item_type_count: self.item_type_count,
            item_id: self.item_id,
            resolved_item_id: self.resolved_item_id,
            item_ref: self.item_ref,
            item_ref_count: self.item_ref_count,
            unresolved_item_refs: self.unresolved_item_refs,
            property_names: self.property_names,
            property_count: self.property_count,
            url_property_count: self.url_property_count,
            structured_data_blocked: self.structured_data_blocked,
            structured_data_block_reasons: self.structured_data_block_reasons,
        }
    }
}

impl ExpectedTemplate {
    fn into_browser_template(self) -> BrowserTemplate {
        BrowserTemplate {
            id: self.id,
            shadowrootmode: self.shadowrootmode,
            shadowrootdelegatesfocus: self.shadowrootdelegatesfocus,
            shadowrootclonable: self.shadowrootclonable,
            shadowrootserializable: self.shadowrootserializable,
            content_text: self.content_text,
        }
    }
}

impl ExpectedTemplateDescriptor {
    fn into_browser_template_descriptor(self) -> BrowserTemplateDescriptor {
        BrowserTemplateDescriptor {
            id: self.id,
            template_kind: self.template_kind,
            shadowrootmode: self.shadowrootmode,
            shadowroot_attribute_names: self.shadowroot_attribute_names,
            shadowroot_attribute_count: self.shadowroot_attribute_count,
            declarative_shadow_root: self.declarative_shadow_root,
            shadowroot_mode_valid: self.shadowroot_mode_valid,
            shadowrootdelegatesfocus: self.shadowrootdelegatesfocus,
            shadowrootclonable: self.shadowrootclonable,
            shadowrootserializable: self.shadowrootserializable,
            content_text: self.content_text,
            content_word_count: self.content_word_count,
            template_blocked: self.template_blocked,
            template_block_reasons: self.template_block_reasons,
        }
    }
}

impl ExpectedSlotDescriptor {
    fn into_browser_slot_descriptor(self) -> BrowserSlotDescriptor {
        BrowserSlotDescriptor {
            element: self.element,
            id: self.id,
            slot_kind: self.slot_kind,
            slot: self.slot,
            slot_name: self.slot_name,
            default_slot: self.default_slot,
            named_slot: self.named_slot,
            fallback_text: self.fallback_text,
            fallback_word_count: self.fallback_word_count,
            part: self.part,
            custom_element: self.custom_element,
            custom_element_name: self.custom_element_name,
            custom_element_is: self.custom_element_is,
            slot_blocked: self.slot_blocked,
            slot_block_reasons: self.slot_block_reasons,
        }
    }
}

impl ExpectedCustomElementDescriptor {
    fn into_browser_custom_element_descriptor(self) -> BrowserCustomElementDescriptor {
        BrowserCustomElementDescriptor {
            element: self.element,
            id: self.id,
            custom_element_kind: self.custom_element_kind,
            definition_name: self.definition_name,
            custom_element_name: self.custom_element_name,
            custom_element_is: self.custom_element_is,
            autonomous_custom_element: self.autonomous_custom_element,
            customized_builtin: self.customized_builtin,
            extends_element: self.extends_element,
            custom_element_name_valid: self.custom_element_name_valid,
            slot: self.slot,
            part: self.part,
            exportparts: self.exportparts,
            data_attribute_names: self.data_attribute_names,
            text: self.text,
            custom_element_blocked: self.custom_element_blocked,
            custom_element_block_reasons: self.custom_element_block_reasons,
        }
    }
}

impl ExpectedCanvasDescriptor {
    fn into_browser_canvas_descriptor(self) -> BrowserCanvasDescriptor {
        BrowserCanvasDescriptor {
            id: self.id,
            classes: self.classes,
            width: self.width,
            height: self.height,
            has_width: self.has_width,
            has_height: self.has_height,
            fallback_text: self.fallback_text,
            fallback_word_count: self.fallback_word_count,
            part: self.part,
            data_attribute_names: self.data_attribute_names,
            event_handlers: self.event_handlers,
            pointer_handlers: self.pointer_handlers,
            keyboard_handlers: self.keyboard_handlers,
            lifecycle_handlers: self.lifecycle_handlers,
            canvas_blocked: self.canvas_blocked,
            canvas_block_reasons: self.canvas_block_reasons,
        }
    }
}

impl ExpectedComponentHydrationTarget {
    fn into_browser_component_hydration_target(self) -> BrowserComponentHydrationTarget {
        BrowserComponentHydrationTarget {
            element: self.element,
            id: self.id,
            classes: self.classes,
            custom_element: self.custom_element,
            custom_element_name: self.custom_element_name,
            custom_element_is: self.custom_element_is,
            slot: self.slot,
            slot_name: self.slot_name,
            part: self.part,
            exportparts: self.exportparts,
            data_attributes: self
                .data_attributes
                .into_iter()
                .map(ExpectedDataAttribute::into_browser_data_attribute)
                .collect(),
            canvas_fallback_text: self.canvas_fallback_text,
            text: self.text,
        }
    }
}

impl ExpectedComponentHydrationDescriptor {
    fn into_browser_component_hydration_descriptor(self) -> BrowserComponentHydrationDescriptor {
        BrowserComponentHydrationDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            hydration_kind: self.hydration_kind,
            custom_element: self.custom_element,
            custom_element_name: self.custom_element_name,
            custom_element_is: self.custom_element_is,
            shadowrootmode: self.shadowrootmode,
            slot: self.slot,
            slot_name: self.slot_name,
            part: self.part,
            exportparts: self.exportparts,
            data_attribute_names: self.data_attribute_names,
            data_attribute_count: self.data_attribute_count,
            canvas_fallback_text: self.canvas_fallback_text,
            text: self.text,
            hydration_blocked: self.hydration_blocked,
            hydration_block_reasons: self.hydration_block_reasons,
        }
    }
}

impl ExpectedDataAttribute {
    fn into_browser_data_attribute(self) -> BrowserDataAttribute {
        BrowserDataAttribute {
            name: self.name,
            value: self.value,
        }
    }
}

impl ExpectedDataAttributeDescriptor {
    fn into_browser_data_attribute_descriptor(self) -> BrowserDataAttributeDescriptor {
        BrowserDataAttributeDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            custom_element: self.custom_element,
            custom_element_name: self.custom_element_name,
            custom_element_is: self.custom_element_is,
            slot: self.slot,
            slot_name: self.slot_name,
            part: self.part,
            data_attributes: self
                .data_attributes
                .into_iter()
                .map(ExpectedDataAttribute::into_browser_data_attribute)
                .collect(),
            data_attribute_names: self.data_attribute_names,
            data_attribute_count: self.data_attribute_count,
            json_data_attribute_names: self.json_data_attribute_names,
            text: self.text,
        }
    }
}

impl ExpectedGlobalStateDescriptor {
    fn into_browser_global_state_descriptor(self) -> BrowserGlobalStateDescriptor {
        BrowserGlobalStateDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            title: self.title,
            lang: self.lang,
            dir: self.dir,
            hidden: self.hidden,
            inert: self.inert,
            tabindex: self.tabindex,
            accesskey: self.accesskey,
            autofocus: self.autofocus,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            draggable: self.draggable,
            draggable_state: self.draggable_state,
            spellcheck: self.spellcheck,
            translate: self.translate,
            global_attribute_names: self.global_attribute_names,
            global_attribute_count: self.global_attribute_count,
            focus_navigation_hint: self.focus_navigation_hint,
            global_state_blocked: self.global_state_blocked,
            global_state_block_reasons: self.global_state_block_reasons,
            text: self.text,
        }
    }
}

impl ExpectedEventHandlerDescriptor {
    fn into_browser_event_handler_descriptor(self) -> BrowserEventHandlerDescriptor {
        BrowserEventHandlerDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            role: self.role,
            source: self.source,
            event_handlers: self.event_handlers,
            handler_count: self.handler_count,
            activation_handlers: self.activation_handlers,
            keyboard_handlers: self.keyboard_handlers,
            pointer_handlers: self.pointer_handlers,
            form_handlers: self.form_handlers,
            media_handlers: self.media_handlers,
            lifecycle_handlers: self.lifecycle_handlers,
            error_handlers: self.error_handlers,
            text: self.text,
        }
    }
}

impl ExpectedLifecycleEventDescriptor {
    fn into_browser_lifecycle_event_descriptor(self) -> BrowserLifecycleEventDescriptor {
        BrowserLifecycleEventDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            role: self.role,
            source: self.source,
            lifecycle_kind: self.lifecycle_kind,
            text: self.text,
            event_handlers: self.event_handlers,
            lifecycle_handlers: self.lifecycle_handlers,
            load_handlers: self.load_handlers,
            unload_handlers: self.unload_handlers,
            visibility_handlers: self.visibility_handlers,
            history_handlers: self.history_handlers,
            network_handlers: self.network_handlers,
            error_handlers: self.error_handlers,
            handler_count: self.handler_count,
            document_scope: self.document_scope,
            body_scope: self.body_scope,
            error_recovery: self.error_recovery,
        }
    }
}

impl ExpectedAnimationInteractionDescriptor {
    fn into_browser_animation_interaction_descriptor(
        self,
    ) -> BrowserAnimationInteractionDescriptor {
        BrowserAnimationInteractionDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            role: self.role,
            source: self.source,
            animation_kind: self.animation_kind,
            text: self.text,
            event_handlers: self.event_handlers,
            animation_handlers: self.animation_handlers,
            animation_start_handlers: self.animation_start_handlers,
            animation_iteration_handlers: self.animation_iteration_handlers,
            animation_end_handlers: self.animation_end_handlers,
            animation_cancel_handlers: self.animation_cancel_handlers,
            transition_handlers: self.transition_handlers,
            transition_run_handlers: self.transition_run_handlers,
            transition_start_handlers: self.transition_start_handlers,
            transition_end_handlers: self.transition_end_handlers,
            transition_cancel_handlers: self.transition_cancel_handlers,
            handler_count: self.handler_count,
            document_scope: self.document_scope,
            body_scope: self.body_scope,
        }
    }
}

impl ExpectedFullscreenInteractionDescriptor {
    fn into_browser_fullscreen_interaction_descriptor(
        self,
    ) -> BrowserFullscreenInteractionDescriptor {
        BrowserFullscreenInteractionDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            role: self.role,
            source: self.source,
            fullscreen_kind: self.fullscreen_kind,
            text: self.text,
            event_handlers: self.event_handlers,
            fullscreen_handlers: self.fullscreen_handlers,
            fullscreen_change_handlers: self.fullscreen_change_handlers,
            fullscreen_error_handlers: self.fullscreen_error_handlers,
            handler_count: self.handler_count,
            allow: self.allow,
            allow_tokens: self.allow_tokens,
            allowfullscreen: self.allowfullscreen,
            fullscreen_allowed: self.fullscreen_allowed,
            embedded_context: self.embedded_context,
            document_scope: self.document_scope,
            body_scope: self.body_scope,
        }
    }
}

impl ExpectedContextMenuInteractionDescriptor {
    fn into_browser_context_menu_interaction_descriptor(
        self,
    ) -> BrowserContextMenuInteractionDescriptor {
        BrowserContextMenuInteractionDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            source: self.source,
            context_menu_kind: self.context_menu_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_haspopup: self.aria_haspopup,
            aria_controls: self.aria_controls,
            aria_expanded: self.aria_expanded,
            popover: self.popover,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            command: self.command,
            command_for: self.command_for,
            event_handlers: self.event_handlers,
            contextmenu_handlers: self.contextmenu_handlers,
            pointer_handlers: self.pointer_handlers,
            keyboard_handlers: self.keyboard_handlers,
            handler_count: self.handler_count,
            focusable: self.focusable,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            context_menu_blocked: self.context_menu_blocked,
            context_menu_block_reasons: self.context_menu_block_reasons,
        }
    }
}

impl ExpectedResource {
    fn into_browser_resource(self) -> BrowserResource {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserResource {
            kind: self.kind,
            url: self.url,
            resolved_url: self.resolved_url,
            rel: self.rel,
            rel_tokens,
            as_hint: self.as_hint,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            sizes: self.sizes,
            hreflang: self.hreflang,
            color: self.color,
            width: self.width,
            height: self.height,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            csp: self.csp,
            blocking: self.blocking,
            blocking_tokens,
            browsing_context_name: self.browsing_context_name,
            loading: self.loading,
            sandbox: self.sandbox,
            allow: self.allow,
            allowfullscreen: self.allowfullscreen,
            srcdoc: self.srcdoc,
            credentialless: self.credentialless,
            imagesrcset: self.imagesrcset,
            resolved_imagesrcset: self.resolved_imagesrcset,
            imagesizes: self.imagesizes,
            track_kind: self.track_kind,
            srclang: self.srclang,
            track_label: self.track_label,
            default_track: self.default_track,
            async_script: self.async_script,
            defer_script: self.defer_script,
        }
    }
}

impl ExpectedLinkResourceDescriptor {
    fn into_browser_link_resource_descriptor(self) -> BrowserLinkResourceDescriptor {
        BrowserLinkResourceDescriptor {
            resource_index: self.resource_index,
            resource_kind: self.resource_kind,
            url: self.url,
            resolved_url: self.resolved_url,
            rel_tokens: self.rel_tokens,
            as_hint: self.as_hint,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            sizes: self.sizes,
            hreflang: self.hreflang,
            color: self.color,
            fetchpriority: self.fetchpriority,
            blocking_tokens: self.blocking_tokens,
            responsive_image_preload: self.responsive_image_preload,
            icon_candidate: self.icon_candidate,
            alternate_candidate: self.alternate_candidate,
            policy_hint_count: self.policy_hint_count,
            resource_blocked: self.resource_blocked,
            resource_block_reasons: self.resource_block_reasons,
        }
    }
}

fn expected_resource_endpoint_descriptors(
    metadata: &BrowserDocumentMetadata,
    resources: &[BrowserResource],
) -> Vec<BrowserResourceEndpointDescriptor> {
    let mut descriptors = Vec::new();
    if let Some(refresh) = &metadata.refresh {
        if let Some(url) = &refresh.url {
            descriptors.push(BrowserResourceEndpointDescriptor {
                endpoint_kind: "metadata-refresh".to_string(),
                element: "meta".to_string(),
                resource_kind: Some("refresh".to_string()),
                url: url.clone(),
                resolved_url: refresh.resolved_url.clone(),
                rel: None,
                rel_tokens: Vec::new(),
                as_hint: None,
                type_hint: None,
                media: None,
                title: None,
                sizes: None,
                hreflang: None,
                color: None,
                width: None,
                height: None,
                integrity: None,
                crossorigin: None,
                nonce: None,
                referrerpolicy: None,
                fetchpriority: None,
                csp: None,
                blocking: None,
                blocking_tokens: Vec::new(),
                browsing_context_name: None,
                loading: None,
                sandbox: Vec::new(),
                allow: None,
                allowfullscreen: false,
                srcdoc: None,
                credentialless: false,
                imagesrcset: None,
                resolved_imagesrcset: None,
                imagesizes: None,
                track_kind: None,
                srclang: None,
                track_label: None,
                default_track: false,
                async_script: false,
                defer_script: false,
            });
        }
    }
    descriptors.extend(resources.iter().map(expected_resource_endpoint_descriptor));
    descriptors
}

fn expected_resource_endpoint_descriptor(
    resource: &BrowserResource,
) -> BrowserResourceEndpointDescriptor {
    BrowserResourceEndpointDescriptor {
        endpoint_kind: "resource".to_string(),
        element: expected_resource_endpoint_element(&resource.kind).to_string(),
        resource_kind: Some(resource.kind.clone()),
        url: resource.url.clone(),
        resolved_url: resource.resolved_url.clone(),
        rel: resource.rel.clone(),
        rel_tokens: resource.rel_tokens.clone(),
        as_hint: resource.as_hint.clone(),
        type_hint: resource.type_hint.clone(),
        media: resource.media.clone(),
        title: resource.title.clone(),
        sizes: resource.sizes.clone(),
        hreflang: resource.hreflang.clone(),
        color: resource.color.clone(),
        width: resource.width.clone(),
        height: resource.height.clone(),
        integrity: resource.integrity.clone(),
        crossorigin: resource.crossorigin.clone(),
        nonce: resource.nonce.clone(),
        referrerpolicy: resource.referrerpolicy.clone(),
        fetchpriority: resource.fetchpriority.clone(),
        csp: resource.csp.clone(),
        blocking: resource.blocking.clone(),
        blocking_tokens: resource.blocking_tokens.clone(),
        browsing_context_name: resource.browsing_context_name.clone(),
        loading: resource.loading.clone(),
        sandbox: resource.sandbox.clone(),
        allow: resource.allow.clone(),
        allowfullscreen: resource.allowfullscreen,
        srcdoc: resource.srcdoc.clone(),
        credentialless: resource.credentialless,
        imagesrcset: resource.imagesrcset.clone(),
        resolved_imagesrcset: resource.resolved_imagesrcset.clone(),
        imagesizes: resource.imagesizes.clone(),
        track_kind: resource.track_kind.clone(),
        srclang: resource.srclang.clone(),
        track_label: resource.track_label.clone(),
        default_track: resource.default_track,
        async_script: resource.async_script,
        defer_script: resource.defer_script,
    }
}

fn expected_resource_endpoint_element(kind: &str) -> &str {
    match kind {
        "alternate" | "canonical" | "dns-prefetch" | "icon" | "link" | "manifest"
        | "modulepreload" | "preconnect" | "prefetch" | "preload" | "prerender" | "stylesheet" => {
            "link"
        }
        "frame" => "iframe",
        "image" => "img",
        other => other,
    }
}

fn expected_media_playback_descriptors(
    media: &[BrowserMedia],
) -> Vec<BrowserMediaPlaybackDescriptor> {
    media
        .iter()
        .map(expected_media_playback_descriptor)
        .collect()
}

fn expected_media_playback_descriptor(media: &BrowserMedia) -> BrowserMediaPlaybackDescriptor {
    BrowserMediaPlaybackDescriptor {
        kind: media.kind.clone(),
        src: media.src.clone(),
        resolved_src: media.resolved_src.clone(),
        poster: media.poster.clone(),
        resolved_poster: media.resolved_poster.clone(),
        width: media.width.clone(),
        height: media.height.clone(),
        controls: media.controls,
        autoplay: media.autoplay,
        loop_media: media.loop_media,
        muted: media.muted,
        playsinline: media.playsinline,
        preload: media.preload.clone(),
        crossorigin: media.crossorigin.clone(),
        controlslist: media.controlslist.clone(),
        controlslist_tokens: media.controlslist_tokens.clone(),
        disableremoteplayback: media.disableremoteplayback,
        disablepictureinpicture: media.disablepictureinpicture,
        source_count: media.sources.len(),
        sources: media.sources.clone(),
        track_count: media.tracks.len(),
        default_track_count: media
            .tracks
            .iter()
            .filter(|track| track.default_track)
            .count(),
        tracks: media.tracks.clone(),
    }
}

fn expected_media_resource_descriptors(
    media: &[BrowserMedia],
) -> Vec<BrowserMediaResourceDescriptor> {
    media
        .iter()
        .enumerate()
        .flat_map(|(media_index, media)| {
            let source_descriptors = media.sources.iter().map(move |source| {
                expected_media_source_resource_descriptor(media_index + 1, media, source)
            });
            let track_descriptors = media.tracks.iter().map(move |track| {
                expected_media_track_resource_descriptor(media_index + 1, media, track)
            });
            source_descriptors.chain(track_descriptors)
        })
        .collect()
}

fn expected_media_source_resource_descriptor(
    media_index: usize,
    media: &BrowserMedia,
    source: &BrowserMediaSource,
) -> BrowserMediaResourceDescriptor {
    let block_reasons = expected_media_source_block_reasons(source);
    BrowserMediaResourceDescriptor {
        media_index,
        media_kind: media.kind.clone(),
        element: "source".to_string(),
        resource_kind: "source".to_string(),
        src: source.src.clone(),
        resolved_src: source.resolved_src.clone(),
        type_hint: source.type_hint.clone(),
        media: source.media.clone(),
        track_kind: None,
        srclang: None,
        label: None,
        default_track: false,
        candidate_kind: if block_reasons.is_empty() {
            "source-candidate".to_string()
        } else {
            "blocked-source".to_string()
        },
        media_resource_blocked: !block_reasons.is_empty(),
        media_resource_block_reasons: block_reasons,
    }
}

fn expected_media_track_resource_descriptor(
    media_index: usize,
    media: &BrowserMedia,
    track: &BrowserMediaTrack,
) -> BrowserMediaResourceDescriptor {
    let block_reasons = expected_media_track_block_reasons(track);
    BrowserMediaResourceDescriptor {
        media_index,
        media_kind: media.kind.clone(),
        element: "track".to_string(),
        resource_kind: "track".to_string(),
        src: track.src.clone(),
        resolved_src: track.resolved_src.clone(),
        type_hint: None,
        media: None,
        track_kind: Some(track.kind.clone()),
        srclang: track.srclang.clone(),
        label: track.label.clone(),
        default_track: track.default_track,
        candidate_kind: if !block_reasons.is_empty() {
            "blocked-track".to_string()
        } else if track.default_track {
            "default-text-track".to_string()
        } else {
            "text-track".to_string()
        },
        media_resource_blocked: !block_reasons.is_empty(),
        media_resource_block_reasons: block_reasons,
    }
}

fn expected_media_source_block_reasons(source: &BrowserMediaSource) -> Vec<String> {
    let mut reasons = Vec::new();
    if source.src.is_none() {
        reasons.push("missing-src".to_string());
    }
    reasons
}

fn expected_media_track_block_reasons(track: &BrowserMediaTrack) -> Vec<String> {
    let mut reasons = Vec::new();
    if track.src.is_none() {
        reasons.push("missing-src".to_string());
    }
    if track.label.is_none() {
        reasons.push("missing-label".to_string());
    }
    if matches!(track.kind.as_str(), "subtitles" | "captions") && track.srclang.is_none() {
        reasons.push("missing-srclang".to_string());
    }
    reasons
}

fn expected_image_candidate_descriptors(
    images: &[BrowserImage],
) -> Vec<BrowserImageCandidateDescriptor> {
    images
        .iter()
        .map(expected_image_candidate_descriptor)
        .collect()
}

fn expected_image_candidate_descriptor(image: &BrowserImage) -> BrowserImageCandidateDescriptor {
    let image_srcset_count = image
        .srcset
        .as_deref()
        .map(expected_srcset_candidate_count)
        .unwrap_or_default();
    let source_srcset_count = image
        .sources
        .iter()
        .filter(|source| source.srcset.is_some())
        .count();
    let source_candidate_count = image
        .sources
        .iter()
        .filter_map(|source| source.srcset.as_deref())
        .map(expected_srcset_candidate_count)
        .sum::<usize>();

    BrowserImageCandidateDescriptor {
        src: image.src.clone(),
        resolved_src: image.resolved_src.clone(),
        srcset: image.srcset.clone(),
        resolved_srcset: image.resolved_srcset.clone(),
        sizes: image.sizes.clone(),
        alt: image.alt.clone(),
        width: image.width.clone(),
        height: image.height.clone(),
        loading: image.loading.clone(),
        decoding: image.decoding.clone(),
        fetchpriority: image.fetchpriority.clone(),
        crossorigin: image.crossorigin.clone(),
        referrerpolicy: image.referrerpolicy.clone(),
        usemap: image.usemap.clone(),
        ismap: image.ismap,
        has_alt: image.alt.is_some(),
        source_count: image.sources.len(),
        source_srcset_count,
        candidate_count: image_srcset_count + source_candidate_count,
        source_type_hints: image
            .sources
            .iter()
            .filter_map(|source| source.type_hint.clone())
            .collect(),
        source_media: image
            .sources
            .iter()
            .filter_map(|source| source.media.clone())
            .collect(),
        sources: image.sources.clone(),
    }
}

fn expected_srcset_candidate_count(srcset: &str) -> usize {
    srcset
        .split(',')
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .count()
}

fn expected_embedded_policy_descriptors(
    contexts: &[BrowserEmbeddedContext],
) -> Vec<BrowserEmbeddedPolicyDescriptor> {
    contexts
        .iter()
        .map(expected_embedded_policy_descriptor)
        .collect()
}

fn expected_embedded_policy_descriptor(
    context: &BrowserEmbeddedContext,
) -> BrowserEmbeddedPolicyDescriptor {
    BrowserEmbeddedPolicyDescriptor {
        element: context.element.clone(),
        resource_kind: expected_embedded_resource_kind(&context.element).to_string(),
        url: context.url.clone(),
        resolved_url: context.resolved_url.clone(),
        browsing_context_name: context.browsing_context_name.clone(),
        title: context.title.clone(),
        type_hint: context.type_hint.clone(),
        width: context.width.clone(),
        height: context.height.clone(),
        loading: context.loading.clone(),
        fetchpriority: context.fetchpriority.clone(),
        csp: context.csp.clone(),
        sandbox: context.sandbox.clone(),
        sandbox_token_count: context.sandbox.len(),
        allow: context.allow.clone(),
        allow_tokens: expected_permission_policy_tokens(context.allow.as_deref()),
        allow_token_count: expected_permission_policy_tokens(context.allow.as_deref()).len(),
        allowfullscreen: context.allowfullscreen,
        fullscreen_allowed: context.allowfullscreen
            || expected_allow_fullscreen_policy(context.allow.as_deref()),
        referrerpolicy: context.referrerpolicy.clone(),
        srcdoc: context.srcdoc.clone(),
        has_srcdoc: context.srcdoc.is_some(),
        credentialless: context.credentialless,
        fallback_text: context.fallback_text.clone(),
    }
}

fn expected_embedded_resource_kind(element: &str) -> &'static str {
    match element {
        "iframe" | "frame" => "document",
        "object" => "object",
        "embed" => "embed",
        _ => "embedded",
    }
}

fn expected_disclosure_state_descriptors(
    disclosures: &[BrowserDisclosure],
) -> Vec<BrowserDisclosureStateDescriptor> {
    disclosures
        .iter()
        .map(expected_disclosure_state_descriptor)
        .collect()
}

fn expected_disclosure_state_descriptor(
    disclosure: &BrowserDisclosure,
) -> BrowserDisclosureStateDescriptor {
    let disclosure_kind = if disclosure.element == "dialog" {
        "dialog"
    } else {
        "details"
    };
    let grouped = disclosure.element == "details" && disclosure.name.is_some();

    BrowserDisclosureStateDescriptor {
        element: disclosure.element.clone(),
        id: disclosure.id.clone(),
        name: disclosure.name.clone(),
        disclosure_kind: disclosure_kind.to_string(),
        open: disclosure.open,
        grouped,
        group_name: grouped.then(|| disclosure.name.clone()).flatten(),
        has_summary: disclosure.summary_text.is_some(),
        summary_text: disclosure.summary_text.clone(),
        accessible_name: disclosure.accessible_name.clone(),
        accessible_description: disclosure.accessible_description.clone(),
        aria_label: disclosure.aria_label.clone(),
        aria_labelledby: disclosure.aria_labelledby.clone(),
        aria_describedby: disclosure.aria_describedby.clone(),
        aria_modal: disclosure.aria_modal.clone(),
        modal: disclosure
            .aria_modal
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
        closedby: disclosure.closedby.clone(),
        text: disclosure.text.clone(),
        text_length: disclosure.text.chars().count(),
    }
}

fn expected_activation_descriptors(
    commands: &[BrowserCommandElement],
    popovers: &[BrowserPopover],
    disclosures: &[BrowserDisclosure],
) -> Vec<BrowserActivationDescriptor> {
    commands
        .iter()
        .map(|command| expected_activation_descriptor(command, popovers, disclosures))
        .collect()
}

fn expected_activation_descriptor(
    command: &BrowserCommandElement,
    popovers: &[BrowserPopover],
    disclosures: &[BrowserDisclosure],
) -> BrowserActivationDescriptor {
    BrowserActivationDescriptor {
        element: command.element.clone(),
        id: command.id.clone(),
        role: command.role.clone(),
        authored_role: command.authored_role.clone(),
        command_kind: command.command_kind.clone(),
        activation_kind: expected_activation_kind(command),
        target_id: expected_activation_target_id(command),
        target_kind: expected_activation_target_kind(command, popovers, disclosures),
        text: command.text.clone(),
        accessible_name: command.accessible_name.clone(),
        accessible_description: command.accessible_description.clone(),
        disabled: command.disabled,
        focusable: command.focusable,
        tabindex: command.tabindex.clone(),
        accesskey: command.accesskey.clone(),
        event_handlers: command.event_handlers.clone(),
        handler_count: command.event_handlers.len(),
        command: command.command.clone(),
        command_for: command.command_for.clone(),
        popover_target: command.popover_target.clone(),
        popover_target_action: command.popover_target_action.clone(),
        aria_controls: command.aria_controls.clone(),
        aria_expanded: command.aria_expanded.clone(),
        aria_haspopup: command.aria_haspopup.clone(),
        aria_pressed: command.aria_pressed.clone(),
        aria_current: command.aria_current.clone(),
        aria_disabled: command.aria_disabled.clone(),
        control_type: command.control_type.clone(),
        href: command.href.clone(),
        resolved_href: command.resolved_href.clone(),
        effective_target: command.effective_target.clone(),
        form_owner: command.form_owner.clone(),
        form_action: command.form_action.clone(),
        resolved_form_action: command.resolved_form_action.clone(),
        form_method: command.form_method.clone(),
        form_target: command.form_target.clone(),
        form_novalidate: command.form_novalidate,
    }
}

fn expected_activation_kind(command: &BrowserCommandElement) -> String {
    if let Some(command_value) = &command.command {
        return command_value.clone();
    }
    if command.popover_target.is_some() {
        let action = command.popover_target_action.as_deref().unwrap_or("toggle");
        return format!("popover-{action}");
    }
    if command.href.is_some() {
        return "navigation".to_string();
    }
    if let Some(control_type) = &command.control_type {
        if matches!(
            control_type.as_str(),
            "submit" | "reset" | "button" | "image"
        ) {
            return format!("form-{control_type}");
        }
    }

    command.command_kind.clone()
}

fn expected_activation_target_id(command: &BrowserCommandElement) -> Option<String> {
    command
        .command_for
        .clone()
        .or_else(|| command.popover_target.clone())
        .or_else(|| command.aria_controls.first().cloned())
}

fn expected_activation_target_kind(
    command: &BrowserCommandElement,
    popovers: &[BrowserPopover],
    disclosures: &[BrowserDisclosure],
) -> String {
    if let Some(command_for) = &command.command_for {
        if let Some(disclosure) = disclosures
            .iter()
            .find(|disclosure| disclosure.id.as_deref() == Some(command_for.as_str()))
        {
            return if disclosure.element == "dialog" {
                "dialog"
            } else {
                "disclosure"
            }
            .to_string();
        }
        if popovers
            .iter()
            .any(|popover| popover.id.as_deref() == Some(command_for.as_str()))
        {
            return "popover".to_string();
        }
        return "command-target".to_string();
    }
    if let Some(popover_target) = &command.popover_target {
        if popovers
            .iter()
            .any(|popover| popover.id.as_deref() == Some(popover_target.as_str()))
        {
            return "popover".to_string();
        }
        return "popover-target".to_string();
    }
    if !command.aria_controls.is_empty() {
        return "controlled-region".to_string();
    }
    if command.href.is_some() {
        return "navigation".to_string();
    }
    if command.form_owner.is_some()
        || command.form_action.is_some()
        || matches!(
            command.control_type.as_deref(),
            Some("submit" | "reset" | "image")
        )
    {
        return "form".to_string();
    }
    if command.command_kind == "disclosure" {
        return "disclosure".to_string();
    }

    "command".to_string()
}

fn expected_focus_navigation_descriptors(
    elements: &[BrowserInteractiveElement],
) -> Vec<BrowserFocusNavigationDescriptor> {
    elements
        .iter()
        .filter(|element| expected_has_focus_navigation_state(element))
        .map(expected_focus_navigation_descriptor)
        .collect()
}

fn expected_has_focus_navigation_state(element: &BrowserInteractiveElement) -> bool {
    element.focusable.is_some()
        || element.tabindex.is_some()
        || !element.accesskey.is_empty()
        || element.contenteditable.is_some()
        || element.disabled
        || element.hidden
        || element.inert
        || element.aria_hidden
        || element.aria_disabled.is_some()
}

fn expected_focus_navigation_descriptor(
    element: &BrowserInteractiveElement,
) -> BrowserFocusNavigationDescriptor {
    let focusable = element.focusable.unwrap_or(false);
    let tabindex_order = element
        .tabindex
        .as_deref()
        .and_then(|tabindex| tabindex.trim().parse::<i32>().ok());
    let focus_block_reasons = expected_focus_block_reasons(element);
    let focus_blocked = !focus_block_reasons.is_empty();
    let sequential_focus = focusable && tabindex_order.unwrap_or(0) >= 0;
    let programmatic_focus = focusable || matches!(tabindex_order, Some(value) if value < 0);

    BrowserFocusNavigationDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        focus_kind: expected_focus_kind(element, focusable, sequential_focus, programmatic_focus),
        focusable,
        sequential_focus,
        programmatic_focus,
        focus_blocked,
        focus_block_reasons,
        tabindex: element.tabindex.clone(),
        tabindex_order,
        accesskey: element.accesskey.clone(),
        event_handlers: element.event_handlers.clone(),
        contenteditable: element.contenteditable.clone(),
        editing_mode: element.editing_mode.clone(),
        command: element.command.clone(),
        command_for: element.command_for.clone(),
        popover_target: element.popover_target.clone(),
        popover_target_action: element.popover_target_action.clone(),
        aria_controls: element.aria_controls.clone(),
        aria_activedescendant: element.aria_activedescendant.clone(),
        aria_expanded: element.aria_expanded.clone(),
        aria_haspopup: element.aria_haspopup.clone(),
        aria_disabled: element.aria_disabled.clone(),
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        text: element.text.clone(),
    }
}

fn expected_focus_kind(
    element: &BrowserInteractiveElement,
    focusable: bool,
    sequential_focus: bool,
    programmatic_focus: bool,
) -> String {
    if !expected_focus_block_reasons(element).is_empty() {
        return "blocked".to_string();
    }
    if element.editing_mode.is_some() {
        return "editing-host".to_string();
    }
    if sequential_focus {
        return "sequential".to_string();
    }
    if programmatic_focus {
        return "programmatic".to_string();
    }
    if !element.accesskey.is_empty() {
        return "accesskey".to_string();
    }
    if focusable {
        return "focusable".to_string();
    }

    "metadata".to_string()
}

fn expected_focus_block_reasons(element: &BrowserInteractiveElement) -> Vec<String> {
    let mut reasons = Vec::new();
    if element.disabled {
        reasons.push("disabled".to_string());
    }
    if element.hidden {
        reasons.push("hidden".to_string());
    }
    if element.inert {
        reasons.push("inert".to_string());
    }
    if element.aria_hidden {
        reasons.push("aria-hidden".to_string());
    }
    if element.aria_disabled.as_deref() == Some("true") {
        reasons.push("aria-disabled".to_string());
    }
    reasons
}

fn expected_keyboard_interaction_descriptors(
    elements: &[BrowserInteractiveElement],
) -> Vec<BrowserKeyboardInteractionDescriptor> {
    elements
        .iter()
        .filter(|element| expected_has_keyboard_interaction_state(element))
        .map(expected_keyboard_interaction_descriptor)
        .collect()
}

fn expected_has_keyboard_interaction_state(element: &BrowserInteractiveElement) -> bool {
    element.focusable.is_some()
        || element.tabindex.is_some()
        || !element.accesskey.is_empty()
        || !element.aria_keyshortcuts.is_empty()
        || !expected_event_handlers_by_kind(&element.event_handlers, expected_keyboard_event)
            .is_empty()
        || element.contenteditable.is_some()
        || element.command.is_some()
        || element.command_for.is_some()
        || element.popover_target.is_some()
        || element.disabled
        || element.hidden
        || element.inert
        || element.aria_hidden
        || element.aria_disabled.is_some()
}

fn expected_keyboard_interaction_descriptor(
    element: &BrowserInteractiveElement,
) -> BrowserKeyboardInteractionDescriptor {
    let focusable = element.focusable.unwrap_or(false);
    let tabindex_order = expected_tabindex_order(element.tabindex.as_deref());
    let sequential_focus = focusable && tabindex_order.unwrap_or(0) >= 0;
    let programmatic_focus = focusable || matches!(tabindex_order, Some(value) if value < 0);
    let keyboard_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_keyboard_event);
    let keyboard_block_reasons = expected_focus_block_reasons(element);
    let keyboard_blocked = !keyboard_block_reasons.is_empty();

    BrowserKeyboardInteractionDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        keyboard_kind: expected_keyboard_kind(element, &keyboard_handlers),
        text: element.text.clone(),
        accessible_name: element.accessible_name.clone(),
        focusable,
        sequential_focus,
        programmatic_focus,
        tabindex: element.tabindex.clone(),
        tabindex_order,
        accesskey: element.accesskey.clone(),
        aria_keyshortcuts: element.aria_keyshortcuts.clone(),
        handler_count: keyboard_handlers.len(),
        keyboard_handlers,
        command: element.command.clone(),
        command_for: element.command_for.clone(),
        popover_target: element.popover_target.clone(),
        popover_target_action: element.popover_target_action.clone(),
        aria_controls: element.aria_controls.clone(),
        aria_activedescendant: element.aria_activedescendant.clone(),
        aria_expanded: element.aria_expanded.clone(),
        aria_haspopup: element.aria_haspopup.clone(),
        aria_disabled: element.aria_disabled.clone(),
        contenteditable: element.contenteditable.clone(),
        editing_mode: element.editing_mode.clone(),
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        keyboard_blocked,
        keyboard_block_reasons,
    }
}

fn expected_keyboard_kind(
    element: &BrowserInteractiveElement,
    keyboard_handlers: &[String],
) -> String {
    if !expected_focus_block_reasons(element).is_empty() {
        return "blocked".to_string();
    }
    if !element.aria_keyshortcuts.is_empty() {
        return "aria-shortcut".to_string();
    }
    if !element.accesskey.is_empty() {
        return "accesskey".to_string();
    }
    if !keyboard_handlers.is_empty() {
        return "keyboard-handler".to_string();
    }
    if element.editing_mode.is_some() {
        return "editing-host".to_string();
    }
    if element.command.is_some()
        || element.command_for.is_some()
        || element.popover_target.is_some()
    {
        return "command".to_string();
    }

    "focus".to_string()
}

fn expected_input_planning_descriptors(
    forms: &[BrowserForm],
    interactive_elements: &[BrowserInteractiveElement],
) -> Vec<BrowserInputPlanningDescriptor> {
    let mut descriptors = Vec::new();
    for form in forms {
        for text_entry in &form.text_entries {
            descriptors.push(expected_input_planning_descriptor_from_text_entry(
                text_entry,
                interactive_elements,
            ));
        }
    }

    for element in interactive_elements {
        if !expected_is_input_editing_host(element) {
            continue;
        }
        if descriptors
            .iter()
            .any(|descriptor| descriptor.id == element.id && descriptor.element == element.element)
        {
            continue;
        }
        descriptors.push(expected_input_planning_descriptor_from_editing_host(
            element,
        ));
    }

    descriptors
}

fn expected_input_planning_descriptor_from_text_entry(
    text_entry: &BrowserFormTextEntry,
    interactive_elements: &[BrowserInteractiveElement],
) -> BrowserInputPlanningDescriptor {
    let matching_interactive = text_entry.id.as_deref().and_then(|id| {
        interactive_elements
            .iter()
            .find(|element| element.id.as_deref() == Some(id))
    });
    let input_handlers = matching_interactive
        .map(|element| {
            expected_event_handlers_by_kind(&element.event_handlers, expected_input_event)
        })
        .unwrap_or_default();
    let mut input_block_reasons = Vec::new();
    if text_entry.disabled {
        input_block_reasons.push("disabled".to_string());
    }
    if text_entry.readonly {
        input_block_reasons.push("readonly".to_string());
    }
    if let Some(reason) = &text_entry.validation_barred_reason {
        input_block_reasons.push(format!("validation-barred:{reason}"));
    }
    if let Some(element) = matching_interactive {
        if element.hidden {
            input_block_reasons.push("hidden".to_string());
        }
        if element.inert {
            input_block_reasons.push("inert".to_string());
        }
        if element.aria_hidden {
            input_block_reasons.push("aria-hidden".to_string());
        }
    }

    BrowserInputPlanningDescriptor {
        element: if text_entry.control_type == "textarea" {
            "textarea".to_string()
        } else {
            "input".to_string()
        },
        id: text_entry.id.clone(),
        input_kind: expected_text_entry_input_kind(text_entry).to_string(),
        control_type: Some(text_entry.control_type.clone()),
        name: text_entry.name.clone(),
        form_owner: text_entry.form_owner.clone(),
        text: text_entry.text.clone(),
        accessible_name: text_entry.accessible_name.clone(),
        accessible_description: text_entry.accessible_description.clone(),
        labels: text_entry.labels.clone(),
        placeholder: text_entry.placeholder.clone(),
        value: text_entry.value.clone(),
        editing_mode: (text_entry.control_type == "textarea").then(|| "plaintext".to_string()),
        autocomplete: text_entry.autocomplete.clone(),
        autocomplete_tokens: text_entry.autocomplete_tokens.clone(),
        autocapitalize: text_entry.autocapitalize.clone(),
        enterkeyhint: text_entry.enterkeyhint.clone(),
        dirname: text_entry.dirname.clone(),
        spellcheck: text_entry.spellcheck.clone(),
        autocorrect: text_entry.autocorrect.clone(),
        inputmode: text_entry.inputmode.clone(),
        pattern: text_entry.pattern.clone(),
        min: text_entry.min.clone(),
        max: text_entry.max.clone(),
        step: text_entry.step.clone(),
        minlength: text_entry.minlength.clone(),
        maxlength: text_entry.maxlength.clone(),
        size: text_entry.size.clone(),
        rows: text_entry.rows.clone(),
        cols: text_entry.cols.clone(),
        wrap: text_entry.wrap.clone(),
        list: text_entry.list.clone(),
        datalist_options: text_entry.datalist_options.clone(),
        focusable: matching_interactive
            .and_then(|element| element.focusable)
            .unwrap_or(!text_entry.disabled),
        input_handlers,
        disabled: text_entry.disabled,
        required: text_entry.required,
        readonly: text_entry.readonly,
        will_validate: text_entry.will_validate,
        validation_attributes: text_entry.validation_attributes.clone(),
        validation_barred_reason: text_entry.validation_barred_reason.clone(),
        hidden: matching_interactive
            .map(|element| element.hidden)
            .unwrap_or(false),
        inert: matching_interactive
            .map(|element| element.inert)
            .unwrap_or(false),
        aria_hidden: matching_interactive
            .map(|element| element.aria_hidden)
            .unwrap_or(false),
        input_blocked: !input_block_reasons.is_empty(),
        input_block_reasons,
    }
}

fn expected_input_planning_descriptor_from_editing_host(
    element: &BrowserInteractiveElement,
) -> BrowserInputPlanningDescriptor {
    let mut input_block_reasons = expected_focus_block_reasons(element);
    if element.aria_disabled.as_deref() == Some("true") {
        input_block_reasons.push("aria-disabled".to_string());
    }
    BrowserInputPlanningDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        input_kind: "editing-host".to_string(),
        control_type: None,
        name: None,
        form_owner: None,
        text: element.text.clone(),
        accessible_name: element.accessible_name.clone(),
        accessible_description: element.accessible_description.clone(),
        labels: Vec::new(),
        placeholder: None,
        value: Some(element.text.clone()),
        editing_mode: element.editing_mode.clone(),
        autocomplete: None,
        autocomplete_tokens: Vec::new(),
        autocapitalize: None,
        enterkeyhint: None,
        dirname: None,
        spellcheck: element.spellcheck.clone(),
        autocorrect: None,
        inputmode: None,
        pattern: None,
        min: None,
        max: None,
        step: None,
        minlength: None,
        maxlength: None,
        size: None,
        rows: None,
        cols: None,
        wrap: None,
        list: None,
        datalist_options: Vec::new(),
        focusable: element.focusable.unwrap_or(false),
        input_handlers: expected_event_handlers_by_kind(
            &element.event_handlers,
            expected_input_event,
        ),
        disabled: element.disabled,
        required: false,
        readonly: false,
        will_validate: false,
        validation_attributes: Vec::new(),
        validation_barred_reason: None,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        input_blocked: !input_block_reasons.is_empty(),
        input_block_reasons,
    }
}

fn expected_is_input_editing_host(element: &BrowserInteractiveElement) -> bool {
    element.contenteditable.is_some()
        || element.editing_mode.is_some()
        || !expected_event_handlers_by_kind(&element.event_handlers, expected_input_event)
            .is_empty()
}

fn expected_text_entry_input_kind(text_entry: &BrowserFormTextEntry) -> &'static str {
    if text_entry.disabled {
        "disabled"
    } else if text_entry.readonly {
        "readonly"
    } else if !text_entry.datalist_options.is_empty() {
        "suggested-text"
    } else if text_entry.control_type == "textarea" {
        "multiline-text"
    } else if text_entry.control_type == "password" {
        "password"
    } else if matches!(text_entry.control_type.as_str(), "email" | "url" | "tel") {
        "contact-text"
    } else if text_entry.control_type == "number" {
        "numeric-text"
    } else if text_entry.required || !text_entry.validation_attributes.is_empty() {
        "constrained-text"
    } else {
        "text"
    }
}

fn expected_content_role(name: &str) -> Option<&'static str> {
    match name {
        "base" | "link" | "meta" | "param" | "script" | "style" | "template" | "title" => None,
        "a" => Some("link"),
        "area" => Some("image_map_area"),
        "img" => Some("image"),
        "map" => Some("image_map"),
        "picture" => Some("picture"),
        "source" => Some("media_source"),
        "track" => Some("media_track"),
        "iframe" | "frame" => Some("frame"),
        "embed" => Some("embed"),
        "object" => Some("object"),
        "audio" | "video" => Some("media"),
        "canvas" => Some("canvas"),
        "slot" => Some("slot"),
        "br" | "wbr" => Some("line_break"),
        "hr" => Some("separator"),
        "blockquote" => Some("quote_block"),
        "q" => Some("quote"),
        "data" => Some("data"),
        "time" => Some("time"),
        "mark" => Some("mark"),
        "ins" => Some("inserted"),
        "del" => Some("deleted"),
        "ruby" => Some("ruby"),
        "rb" => Some("ruby_base"),
        "rt" => Some("ruby_text"),
        "rp" => Some("ruby_fallback"),
        "rtc" => Some("ruby_text_container"),
        "bdi" => Some("bidi_isolate"),
        "bdo" => Some("bidi_override"),
        "figure" => Some("figure"),
        "figcaption" => Some("figure_caption"),
        "details" => Some("disclosure"),
        "summary" => Some("disclosure_summary"),
        "dialog" => Some("dialog"),
        "search" => Some("search"),
        "address" => Some("contact"),
        "dl" => Some("description_list"),
        "dt" => Some("description_term"),
        "dd" => Some("description_details"),
        "p" => Some("paragraph"),
        "pre" | "plaintext" | "xmp" | "listing" => Some("preformatted"),
        "article" => Some("article"),
        "aside" => Some("aside"),
        "footer" => Some("footer"),
        "header" => Some("header"),
        "hgroup" => Some("heading_group"),
        "main" => Some("main"),
        "nav" => Some("navigation"),
        "section" => Some("section"),
        "form" => Some("form"),
        "fieldset" => Some("form_group"),
        "label" => Some("label"),
        "legend" => Some("legend"),
        "meter" => Some("meter"),
        "progress" => Some("progress"),
        "input" | "button" | "select" | "textarea" => Some("control"),
        "optgroup" => Some("option_group"),
        "option" => Some("option"),
        "ul" | "ol" | "menu" | "dir" => Some("list"),
        "li" => Some("list_item"),
        "table" => Some("table"),
        "caption" => Some("table_caption"),
        "colgroup" => Some("table_column_group"),
        "col" => Some("table_column"),
        "tbody" | "thead" | "tfoot" => Some("table_section"),
        "tr" => Some("table_row"),
        "td" | "th" => Some("table_cell"),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some("heading"),
        name if expected_is_browser_block_element(name) => Some("block"),
        _ => Some("inline"),
    }
}

fn expected_is_browser_block_element(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "body"
            | "center"
            | "details"
            | "dialog"
            | "div"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "header"
            | "hgroup"
            | "hr"
            | "html"
            | "legend"
            | "main"
            | "nav"
            | "p"
            | "plaintext"
            | "pre"
            | "section"
            | "summary"
            | "xmp"
    )
}

fn expected_drag_drop_descriptors(
    interactive_elements: &[BrowserInteractiveElement],
    global_state_descriptors: &[BrowserGlobalStateDescriptor],
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserDragDropDescriptor> {
    let mut descriptors = Vec::new();

    for element in interactive_elements {
        let event_descriptor = expected_matching_event_descriptor(
            event_handler_descriptors,
            &element.element,
            element.id.as_deref(),
        );
        if expected_interactive_has_drag_drop_state(element, event_descriptor) {
            descriptors.push(expected_drag_drop_descriptor_from_interactive(
                element,
                event_descriptor,
            ));
        }
    }

    for global in global_state_descriptors {
        if descriptors
            .iter()
            .any(|descriptor| descriptor.element == global.element && descriptor.id == global.id)
        {
            continue;
        }
        let event_descriptor = expected_matching_event_descriptor(
            event_handler_descriptors,
            &global.element,
            global.id.as_deref(),
        );
        if expected_global_has_drag_drop_state(global, event_descriptor) {
            descriptors.push(expected_drag_drop_descriptor_from_global(
                global,
                event_descriptor,
            ));
        }
    }

    for event_descriptor in event_handler_descriptors {
        if event_descriptor.source != "element" {
            continue;
        }
        if descriptors.iter().any(|descriptor| {
            descriptor.element == event_descriptor.element && descriptor.id == event_descriptor.id
        }) {
            continue;
        }
        if expected_event_descriptor_has_drag_drop_state(event_descriptor) {
            descriptors.push(expected_drag_drop_descriptor_from_event(event_descriptor));
        }
    }

    descriptors
}

fn expected_matching_event_descriptor<'a>(
    event_descriptors: &'a [BrowserEventHandlerDescriptor],
    element: &str,
    id: Option<&str>,
) -> Option<&'a BrowserEventHandlerDescriptor> {
    event_descriptors.iter().find(|descriptor| {
        descriptor.source == "element"
            && descriptor.element == element
            && descriptor.id.as_deref() == id
    })
}

fn expected_interactive_has_drag_drop_state(
    element: &BrowserInteractiveElement,
    event_descriptor: Option<&BrowserEventHandlerDescriptor>,
) -> bool {
    element.draggable.is_some()
        || event_descriptor
            .map(expected_event_descriptor_has_drag_drop_state)
            .unwrap_or(false)
}

fn expected_global_has_drag_drop_state(
    global: &BrowserGlobalStateDescriptor,
    event_descriptor: Option<&BrowserEventHandlerDescriptor>,
) -> bool {
    global.draggable.is_some()
        || event_descriptor
            .map(expected_event_descriptor_has_drag_drop_state)
            .unwrap_or(false)
}

fn expected_event_descriptor_has_drag_drop_state(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_drag_event)
        .is_empty()
        || !expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_drop_event)
            .is_empty()
}

fn expected_drag_drop_descriptor_from_interactive(
    element: &BrowserInteractiveElement,
    event_descriptor: Option<&BrowserEventHandlerDescriptor>,
) -> BrowserDragDropDescriptor {
    let drag_handlers = event_descriptor
        .map(|descriptor| {
            expected_event_handlers_by_kind(&descriptor.event_handlers, expected_drag_event)
        })
        .unwrap_or_default();
    let drop_handlers = event_descriptor
        .map(|descriptor| {
            expected_event_handlers_by_kind(&descriptor.event_handlers, expected_drop_event)
        })
        .unwrap_or_default();
    let pointer_handlers = event_descriptor
        .map(|descriptor| descriptor.pointer_handlers.clone())
        .unwrap_or_default();
    let drag_block_reasons = expected_drag_block_reasons_for_interactive(element);
    let drag_source = expected_draggable_source(element.draggable_state.as_deref());
    let drop_target = !drop_handlers.is_empty();

    BrowserDragDropDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        classes: Vec::new(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        drag_kind: expected_drag_kind(
            drag_source,
            drop_target,
            &drag_handlers,
            &pointer_handlers,
            &drag_block_reasons,
        ),
        text: element.text.clone(),
        draggable: element.draggable.clone(),
        draggable_state: element.draggable_state.clone(),
        drag_source,
        drop_target,
        handler_count: drag_handlers.len() + drop_handlers.len(),
        drag_handlers,
        drop_handlers,
        pointer_handlers,
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        drag_blocked: !drag_block_reasons.is_empty(),
        drag_block_reasons,
    }
}

fn expected_drag_drop_descriptor_from_global(
    global: &BrowserGlobalStateDescriptor,
    event_descriptor: Option<&BrowserEventHandlerDescriptor>,
) -> BrowserDragDropDescriptor {
    let drag_handlers = event_descriptor
        .map(|descriptor| {
            expected_event_handlers_by_kind(&descriptor.event_handlers, expected_drag_event)
        })
        .unwrap_or_default();
    let drop_handlers = event_descriptor
        .map(|descriptor| {
            expected_event_handlers_by_kind(&descriptor.event_handlers, expected_drop_event)
        })
        .unwrap_or_default();
    let pointer_handlers = event_descriptor
        .map(|descriptor| descriptor.pointer_handlers.clone())
        .unwrap_or_default();
    let drag_block_reasons = expected_drag_block_reasons_for_global(global);
    let drag_source = expected_draggable_source(global.draggable_state.as_deref());
    let drop_target = !drop_handlers.is_empty();

    BrowserDragDropDescriptor {
        element: global.element.clone(),
        id: global.id.clone(),
        classes: global.classes.clone(),
        role: expected_content_role(&global.element).map(ToOwned::to_owned),
        authored_role: None,
        drag_kind: expected_drag_kind(
            drag_source,
            drop_target,
            &drag_handlers,
            &pointer_handlers,
            &drag_block_reasons,
        ),
        text: global.text.clone(),
        draggable: global.draggable.clone(),
        draggable_state: global.draggable_state.clone(),
        drag_source,
        drop_target,
        handler_count: drag_handlers.len() + drop_handlers.len(),
        drag_handlers,
        drop_handlers,
        pointer_handlers,
        disabled: false,
        hidden: global.hidden,
        inert: global.inert,
        aria_hidden: false,
        drag_blocked: !drag_block_reasons.is_empty(),
        drag_block_reasons,
    }
}

fn expected_drag_drop_descriptor_from_event(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserDragDropDescriptor {
    let drag_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_drag_event);
    let drop_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_drop_event);
    let drag_block_reasons = Vec::new();
    let drag_source = false;
    let drop_target = !drop_handlers.is_empty();

    BrowserDragDropDescriptor {
        element: event_descriptor.element.clone(),
        id: event_descriptor.id.clone(),
        classes: event_descriptor.classes.clone(),
        role: event_descriptor.role.clone(),
        authored_role: None,
        drag_kind: expected_drag_kind(
            drag_source,
            drop_target,
            &drag_handlers,
            &event_descriptor.pointer_handlers,
            &drag_block_reasons,
        ),
        text: event_descriptor.text.clone(),
        draggable: None,
        draggable_state: None,
        drag_source,
        drop_target,
        handler_count: drag_handlers.len() + drop_handlers.len(),
        drag_handlers,
        drop_handlers,
        pointer_handlers: event_descriptor.pointer_handlers.clone(),
        disabled: false,
        hidden: false,
        inert: false,
        aria_hidden: false,
        drag_blocked: false,
        drag_block_reasons,
    }
}

fn expected_drag_block_reasons_for_interactive(element: &BrowserInteractiveElement) -> Vec<String> {
    let mut reasons = Vec::new();
    if element.disabled {
        reasons.push("disabled".to_string());
    }
    if element.hidden {
        reasons.push("hidden".to_string());
    }
    if element.inert {
        reasons.push("inert".to_string());
    }
    if element.aria_hidden {
        reasons.push("aria-hidden".to_string());
    }
    if element.aria_disabled.as_deref() == Some("true") {
        reasons.push("aria-disabled".to_string());
    }
    reasons
}

fn expected_drag_block_reasons_for_global(global: &BrowserGlobalStateDescriptor) -> Vec<String> {
    let mut reasons = Vec::new();
    if global.hidden {
        reasons.push("hidden".to_string());
    }
    if global.inert {
        reasons.push("inert".to_string());
    }
    reasons
}

fn expected_draggable_source(draggable_state: Option<&str>) -> bool {
    matches!(draggable_state, Some("true") | Some("auto"))
}

fn expected_drag_kind(
    drag_source: bool,
    drop_target: bool,
    drag_handlers: &[String],
    pointer_handlers: &[String],
    drag_block_reasons: &[String],
) -> String {
    if !drag_block_reasons.is_empty() {
        "blocked".to_string()
    } else if drag_source && drop_target {
        "drag-source-and-drop-target".to_string()
    } else if drag_source {
        "drag-source".to_string()
    } else if drop_target {
        "drop-target".to_string()
    } else if !drag_handlers.is_empty() {
        "drag-handler".to_string()
    } else if !pointer_handlers.is_empty() {
        "pointer-handler".to_string()
    } else {
        "metadata".to_string()
    }
}

fn expected_clipboard_interaction_descriptors(
    forms: &[BrowserForm],
    interactive_elements: &[BrowserInteractiveElement],
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserClipboardInteractionDescriptor> {
    let mut descriptors = Vec::new();

    for form in forms {
        for text_entry in &form.text_entries {
            let matching_interactive = text_entry.id.as_deref().and_then(|id| {
                interactive_elements
                    .iter()
                    .find(|element| element.id.as_deref() == Some(id))
            });
            if expected_text_entry_has_clipboard_state(text_entry, matching_interactive) {
                descriptors.push(expected_clipboard_descriptor_from_text_entry(
                    text_entry,
                    matching_interactive,
                ));
            }
        }
    }

    for element in interactive_elements {
        if descriptors
            .iter()
            .any(|descriptor| descriptor.element == element.element && descriptor.id == element.id)
        {
            continue;
        }
        if expected_interactive_has_clipboard_state(element) {
            descriptors.push(expected_clipboard_descriptor_from_interactive(element));
        }
    }

    for event_descriptor in event_handler_descriptors {
        if event_descriptor.source != "element" {
            continue;
        }
        if descriptors.iter().any(|descriptor| {
            descriptor.element == event_descriptor.element && descriptor.id == event_descriptor.id
        }) {
            continue;
        }
        if expected_event_descriptor_has_clipboard_state(event_descriptor) {
            descriptors.push(expected_clipboard_descriptor_from_event(event_descriptor));
        }
    }

    descriptors
}

fn expected_text_entry_has_clipboard_state(
    text_entry: &BrowserFormTextEntry,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> bool {
    text_entry.disabled
        || text_entry.readonly
        || matching_interactive
            .map(|element| {
                !expected_event_handlers_by_kind(&element.event_handlers, expected_clipboard_event)
                    .is_empty()
                    || !expected_event_handlers_by_kind(
                        &element.event_handlers,
                        expected_input_event,
                    )
                    .is_empty()
                    || element.hidden
                    || element.inert
                    || element.aria_hidden
            })
            .unwrap_or(false)
}

fn expected_interactive_has_clipboard_state(element: &BrowserInteractiveElement) -> bool {
    element.contenteditable.is_some()
        || element.editing_mode.is_some()
        || !expected_event_handlers_by_kind(&element.event_handlers, expected_clipboard_event)
            .is_empty()
}

fn expected_event_descriptor_has_clipboard_state(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_clipboard_event)
        .is_empty()
}

fn expected_clipboard_descriptor_from_text_entry(
    text_entry: &BrowserFormTextEntry,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> BrowserClipboardInteractionDescriptor {
    let event_handlers = matching_interactive
        .map(|element| element.event_handlers.as_slice())
        .unwrap_or(&[]);
    let clipboard_handlers =
        expected_event_handlers_by_kind(event_handlers, expected_clipboard_event);
    let copy_handlers = expected_event_handlers_by_kind(event_handlers, expected_copy_event);
    let cut_handlers = expected_event_handlers_by_kind(event_handlers, expected_cut_event);
    let paste_handlers = expected_event_handlers_by_kind(event_handlers, expected_paste_event);
    let input_handlers = expected_event_handlers_by_kind(event_handlers, expected_input_event);
    let clipboard_block_reasons =
        expected_clipboard_block_reasons_for_text_entry(text_entry, matching_interactive);

    BrowserClipboardInteractionDescriptor {
        element: if text_entry.control_type == "textarea" {
            "textarea".to_string()
        } else {
            "input".to_string()
        },
        id: text_entry.id.clone(),
        role: Some("control".to_string()),
        authored_role: matching_interactive.and_then(|element| element.authored_role.clone()),
        clipboard_kind: expected_clipboard_kind(
            text_entry.readonly,
            text_entry.control_type == "textarea",
            false,
            &copy_handlers,
            &cut_handlers,
            &paste_handlers,
            &input_handlers,
            &clipboard_block_reasons,
        ),
        text: text_entry.text.clone(),
        accessible_name: text_entry.accessible_name.clone(),
        control_type: Some(text_entry.control_type.clone()),
        name: text_entry.name.clone(),
        form_owner: text_entry.form_owner.clone(),
        value: text_entry.value.clone(),
        contenteditable: None,
        editing_mode: (text_entry.control_type == "textarea").then(|| "plaintext".to_string()),
        spellcheck: text_entry.spellcheck.clone(),
        handler_count: clipboard_handlers.len() + input_handlers.len(),
        clipboard_handlers,
        copy_handlers,
        cut_handlers,
        paste_handlers,
        input_handlers,
        focusable: matching_interactive
            .and_then(|element| element.focusable)
            .unwrap_or(!text_entry.disabled),
        readonly: text_entry.readonly,
        disabled: text_entry.disabled,
        hidden: matching_interactive
            .map(|element| element.hidden)
            .unwrap_or(false),
        inert: matching_interactive
            .map(|element| element.inert)
            .unwrap_or(false),
        aria_hidden: matching_interactive
            .map(|element| element.aria_hidden)
            .unwrap_or(false),
        clipboard_blocked: !clipboard_block_reasons.is_empty(),
        clipboard_block_reasons,
    }
}

fn expected_clipboard_descriptor_from_interactive(
    element: &BrowserInteractiveElement,
) -> BrowserClipboardInteractionDescriptor {
    let clipboard_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_clipboard_event);
    let copy_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_copy_event);
    let cut_handlers = expected_event_handlers_by_kind(&element.event_handlers, expected_cut_event);
    let paste_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_paste_event);
    let input_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_input_event);
    let clipboard_block_reasons = expected_clipboard_block_reasons_for_interactive(element);

    BrowserClipboardInteractionDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        clipboard_kind: expected_clipboard_kind(
            false,
            false,
            element.editing_mode.is_some(),
            &copy_handlers,
            &cut_handlers,
            &paste_handlers,
            &input_handlers,
            &clipboard_block_reasons,
        ),
        text: element.text.clone(),
        accessible_name: element.accessible_name.clone(),
        control_type: None,
        name: None,
        form_owner: None,
        value: element.editing_mode.is_some().then(|| element.text.clone()),
        contenteditable: element.contenteditable.clone(),
        editing_mode: element.editing_mode.clone(),
        spellcheck: element.spellcheck.clone(),
        handler_count: clipboard_handlers.len() + input_handlers.len(),
        clipboard_handlers,
        copy_handlers,
        cut_handlers,
        paste_handlers,
        input_handlers,
        focusable: element.focusable.unwrap_or(false),
        readonly: false,
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        clipboard_blocked: !clipboard_block_reasons.is_empty(),
        clipboard_block_reasons,
    }
}

fn expected_clipboard_descriptor_from_event(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserClipboardInteractionDescriptor {
    let clipboard_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_clipboard_event);
    let copy_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_copy_event);
    let cut_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_cut_event);
    let paste_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_paste_event);
    let input_handlers = Vec::new();
    let clipboard_block_reasons = Vec::new();

    BrowserClipboardInteractionDescriptor {
        element: event_descriptor.element.clone(),
        id: event_descriptor.id.clone(),
        role: event_descriptor.role.clone(),
        authored_role: None,
        clipboard_kind: expected_clipboard_kind(
            false,
            false,
            false,
            &copy_handlers,
            &cut_handlers,
            &paste_handlers,
            &input_handlers,
            &clipboard_block_reasons,
        ),
        text: event_descriptor.text.clone(),
        accessible_name: None,
        control_type: None,
        name: None,
        form_owner: None,
        value: None,
        contenteditable: None,
        editing_mode: None,
        spellcheck: None,
        handler_count: clipboard_handlers.len(),
        clipboard_handlers,
        copy_handlers,
        cut_handlers,
        paste_handlers,
        input_handlers,
        focusable: false,
        readonly: false,
        disabled: false,
        hidden: false,
        inert: false,
        aria_hidden: false,
        clipboard_blocked: false,
        clipboard_block_reasons,
    }
}

fn expected_clipboard_block_reasons_for_text_entry(
    text_entry: &BrowserFormTextEntry,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if text_entry.disabled {
        reasons.push("disabled".to_string());
    }
    if text_entry.readonly {
        reasons.push("readonly".to_string());
    }
    if let Some(element) = matching_interactive {
        reasons.extend(expected_clipboard_block_reasons_for_interactive(element));
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn expected_clipboard_block_reasons_for_interactive(
    element: &BrowserInteractiveElement,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if element.disabled {
        reasons.push("disabled".to_string());
    }
    if element.hidden {
        reasons.push("hidden".to_string());
    }
    if element.inert {
        reasons.push("inert".to_string());
    }
    if element.aria_hidden {
        reasons.push("aria-hidden".to_string());
    }
    if element.aria_disabled.as_deref() == Some("true") {
        reasons.push("aria-disabled".to_string());
    }
    reasons
}

fn expected_clipboard_kind(
    readonly: bool,
    multiline: bool,
    editing_host: bool,
    copy_handlers: &[String],
    cut_handlers: &[String],
    paste_handlers: &[String],
    input_handlers: &[String],
    clipboard_block_reasons: &[String],
) -> String {
    if !clipboard_block_reasons.is_empty() {
        "blocked".to_string()
    } else if !paste_handlers.is_empty() {
        "paste-target".to_string()
    } else if !cut_handlers.is_empty() {
        "cut-target".to_string()
    } else if !copy_handlers.is_empty() {
        "copy-source".to_string()
    } else if !input_handlers.is_empty() {
        "input-editor".to_string()
    } else if editing_host {
        "editing-host".to_string()
    } else if readonly {
        "readonly-text".to_string()
    } else if multiline {
        "multiline-text".to_string()
    } else {
        "text-control".to_string()
    }
}

fn expected_selection_interaction_descriptors(
    forms: &[BrowserForm],
    interactive_elements: &[BrowserInteractiveElement],
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserSelectionInteractionDescriptor> {
    let mut descriptors = Vec::new();

    for form in forms {
        for text_entry in &form.text_entries {
            let matching_interactive = text_entry.id.as_deref().and_then(|id| {
                interactive_elements
                    .iter()
                    .find(|element| element.id.as_deref() == Some(id))
            });
            if expected_text_entry_has_selection_state(text_entry, matching_interactive) {
                descriptors.push(expected_selection_descriptor_from_text_entry(
                    text_entry,
                    matching_interactive,
                ));
            }
        }
    }

    for element in interactive_elements {
        if descriptors
            .iter()
            .any(|descriptor| descriptor.element == element.element && descriptor.id == element.id)
        {
            continue;
        }
        if expected_interactive_has_selection_state(element) {
            descriptors.push(expected_selection_descriptor_from_interactive(element));
        }
    }

    for event_descriptor in event_handler_descriptors {
        if descriptors.iter().any(|descriptor| {
            descriptor.element == event_descriptor.element && descriptor.id == event_descriptor.id
        }) {
            continue;
        }
        if expected_event_descriptor_has_selection_state(event_descriptor) {
            descriptors.push(expected_selection_descriptor_from_event(event_descriptor));
        }
    }

    descriptors
}

fn expected_text_entry_has_selection_state(
    text_entry: &BrowserFormTextEntry,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> bool {
    text_entry.disabled
        || text_entry.readonly
        || matching_interactive
            .map(|element| {
                !expected_event_handlers_by_kind(&element.event_handlers, expected_selection_event)
                    .is_empty()
                    || !expected_event_handlers_by_kind(
                        &element.event_handlers,
                        expected_selection_input_event,
                    )
                    .is_empty()
                    || element.hidden
                    || element.inert
                    || element.aria_hidden
            })
            .unwrap_or(false)
}

fn expected_interactive_has_selection_state(element: &BrowserInteractiveElement) -> bool {
    element.contenteditable.is_some()
        || element.editing_mode.is_some()
        || !expected_event_handlers_by_kind(&element.event_handlers, expected_selection_event)
            .is_empty()
}

fn expected_event_descriptor_has_selection_state(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_selection_event)
        .is_empty()
}

fn expected_selection_descriptor_from_text_entry(
    text_entry: &BrowserFormTextEntry,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> BrowserSelectionInteractionDescriptor {
    let event_handlers = matching_interactive
        .map(|element| element.event_handlers.as_slice())
        .unwrap_or(&[]);
    let selection_handlers =
        expected_event_handlers_by_kind(event_handlers, expected_selection_event);
    let select_handlers = expected_event_handlers_by_kind(event_handlers, expected_select_event);
    let selection_change_handlers =
        expected_event_handlers_by_kind(event_handlers, expected_selection_change_event);
    let input_handlers =
        expected_event_handlers_by_kind(event_handlers, expected_selection_input_event);
    let selection_block_reasons =
        expected_selection_block_reasons_for_text_entry(text_entry, matching_interactive);

    BrowserSelectionInteractionDescriptor {
        element: if text_entry.control_type == "textarea" {
            "textarea".to_string()
        } else {
            "input".to_string()
        },
        id: text_entry.id.clone(),
        role: Some("control".to_string()),
        authored_role: matching_interactive.and_then(|element| element.authored_role.clone()),
        selection_kind: expected_selection_kind(
            text_entry.readonly,
            text_entry.control_type == "textarea",
            false,
            &select_handlers,
            &selection_change_handlers,
            &input_handlers,
            &selection_block_reasons,
        ),
        text: text_entry.text.clone(),
        accessible_name: text_entry.accessible_name.clone(),
        control_type: Some(text_entry.control_type.clone()),
        name: text_entry.name.clone(),
        form_owner: text_entry.form_owner.clone(),
        value: text_entry.value.clone(),
        contenteditable: None,
        editing_mode: (text_entry.control_type == "textarea").then(|| "plaintext".to_string()),
        spellcheck: text_entry.spellcheck.clone(),
        handler_count: selection_handlers.len() + input_handlers.len(),
        selection_handlers,
        select_handlers,
        selection_change_handlers,
        input_handlers,
        focusable: matching_interactive
            .and_then(|element| element.focusable)
            .unwrap_or(!text_entry.disabled),
        readonly: text_entry.readonly,
        disabled: text_entry.disabled,
        hidden: matching_interactive
            .map(|element| element.hidden)
            .unwrap_or(false),
        inert: matching_interactive
            .map(|element| element.inert)
            .unwrap_or(false),
        aria_hidden: matching_interactive
            .map(|element| element.aria_hidden)
            .unwrap_or(false),
        selection_blocked: !selection_block_reasons.is_empty(),
        selection_block_reasons,
    }
}

fn expected_selection_descriptor_from_interactive(
    element: &BrowserInteractiveElement,
) -> BrowserSelectionInteractionDescriptor {
    let selection_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_selection_event);
    let select_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_select_event);
    let selection_change_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_selection_change_event);
    let input_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_selection_input_event);
    let selection_block_reasons = expected_selection_block_reasons_for_interactive(element);

    BrowserSelectionInteractionDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        selection_kind: expected_selection_kind(
            false,
            false,
            element.editing_mode.is_some(),
            &select_handlers,
            &selection_change_handlers,
            &input_handlers,
            &selection_block_reasons,
        ),
        text: element.text.clone(),
        accessible_name: element.accessible_name.clone(),
        control_type: None,
        name: None,
        form_owner: None,
        value: element.editing_mode.is_some().then(|| element.text.clone()),
        contenteditable: element.contenteditable.clone(),
        editing_mode: element.editing_mode.clone(),
        spellcheck: element.spellcheck.clone(),
        handler_count: selection_handlers.len() + input_handlers.len(),
        selection_handlers,
        select_handlers,
        selection_change_handlers,
        input_handlers,
        focusable: element.focusable.unwrap_or(false),
        readonly: false,
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        selection_blocked: !selection_block_reasons.is_empty(),
        selection_block_reasons,
    }
}

fn expected_selection_descriptor_from_event(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserSelectionInteractionDescriptor {
    let selection_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_selection_event);
    let select_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_select_event);
    let selection_change_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_selection_change_event,
    );
    let input_handlers = Vec::new();
    let selection_block_reasons = Vec::new();

    BrowserSelectionInteractionDescriptor {
        element: event_descriptor.element.clone(),
        id: event_descriptor.id.clone(),
        role: event_descriptor.role.clone(),
        authored_role: None,
        selection_kind: expected_selection_kind(
            false,
            false,
            false,
            &select_handlers,
            &selection_change_handlers,
            &input_handlers,
            &selection_block_reasons,
        ),
        text: event_descriptor.text.clone(),
        accessible_name: None,
        control_type: None,
        name: None,
        form_owner: None,
        value: None,
        contenteditable: None,
        editing_mode: None,
        spellcheck: None,
        handler_count: selection_handlers.len(),
        selection_handlers,
        select_handlers,
        selection_change_handlers,
        input_handlers,
        focusable: false,
        readonly: false,
        disabled: false,
        hidden: false,
        inert: false,
        aria_hidden: false,
        selection_blocked: false,
        selection_block_reasons,
    }
}

fn expected_selection_block_reasons_for_text_entry(
    text_entry: &BrowserFormTextEntry,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if text_entry.disabled {
        reasons.push("disabled".to_string());
    }
    if text_entry.readonly {
        reasons.push("readonly".to_string());
    }
    if let Some(element) = matching_interactive {
        reasons.extend(expected_selection_block_reasons_for_interactive(element));
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn expected_selection_block_reasons_for_interactive(
    element: &BrowserInteractiveElement,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if element.disabled {
        reasons.push("disabled".to_string());
    }
    if element.hidden {
        reasons.push("hidden".to_string());
    }
    if element.inert {
        reasons.push("inert".to_string());
    }
    if element.aria_hidden {
        reasons.push("aria-hidden".to_string());
    }
    if element.aria_disabled.as_deref() == Some("true") {
        reasons.push("aria-disabled".to_string());
    }
    reasons
}

fn expected_selection_kind(
    readonly: bool,
    multiline: bool,
    editing_host: bool,
    select_handlers: &[String],
    selection_change_handlers: &[String],
    input_handlers: &[String],
    selection_block_reasons: &[String],
) -> String {
    if !selection_block_reasons.is_empty() {
        "blocked".to_string()
    } else if !selection_change_handlers.is_empty() {
        "selection-change".to_string()
    } else if !select_handlers.is_empty() {
        "select-handler".to_string()
    } else if editing_host {
        "editing-host".to_string()
    } else if readonly {
        "readonly-text".to_string()
    } else if multiline {
        "multiline-text".to_string()
    } else if !input_handlers.is_empty() {
        "input-selection".to_string()
    } else {
        "text-control".to_string()
    }
}

fn expected_composition_interaction_descriptors(
    forms: &[BrowserForm],
    interactive_elements: &[BrowserInteractiveElement],
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserCompositionInteractionDescriptor> {
    let mut descriptors = Vec::new();

    for form in forms {
        for text_entry in &form.text_entries {
            let matching_interactive = text_entry.id.as_deref().and_then(|id| {
                interactive_elements
                    .iter()
                    .find(|element| element.id.as_deref() == Some(id))
            });
            if expected_text_entry_has_composition_state(text_entry, matching_interactive) {
                descriptors.push(expected_composition_descriptor_from_text_entry(
                    text_entry,
                    matching_interactive,
                ));
            }
        }
    }

    for element in interactive_elements {
        if descriptors
            .iter()
            .any(|descriptor| descriptor.element == element.element && descriptor.id == element.id)
        {
            continue;
        }
        if expected_interactive_has_composition_state(element) {
            descriptors.push(expected_composition_descriptor_from_interactive(element));
        }
    }

    for event_descriptor in event_handler_descriptors {
        if descriptors.iter().any(|descriptor| {
            descriptor.element == event_descriptor.element
                && descriptor.id == event_descriptor.id
                && (event_descriptor.source == "element"
                    || descriptor.source == event_descriptor.source)
        }) {
            continue;
        }
        if expected_event_descriptor_has_composition_state(event_descriptor) {
            descriptors.push(expected_composition_descriptor_from_event(event_descriptor));
        }
    }

    descriptors
}

fn expected_text_entry_has_composition_state(
    text_entry: &BrowserFormTextEntry,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> bool {
    text_entry.disabled
        || text_entry.readonly
        || matching_interactive
            .map(|element| {
                !expected_event_handlers_by_kind(
                    &element.event_handlers,
                    expected_composition_event,
                )
                .is_empty()
                    || !expected_event_handlers_by_kind(
                        &element.event_handlers,
                        expected_text_input_event,
                    )
                    .is_empty()
                    || element.hidden
                    || element.inert
                    || element.aria_hidden
            })
            .unwrap_or(false)
}

fn expected_interactive_has_composition_state(element: &BrowserInteractiveElement) -> bool {
    element.contenteditable.is_some()
        || element.editing_mode.is_some()
        || !expected_event_handlers_by_kind(&element.event_handlers, expected_composition_event)
            .is_empty()
        || !expected_event_handlers_by_kind(&element.event_handlers, expected_text_input_event)
            .is_empty()
}

fn expected_event_descriptor_has_composition_state(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_composition_event)
        .is_empty()
        || !expected_event_handlers_by_kind(
            &event_descriptor.event_handlers,
            expected_text_input_event,
        )
        .is_empty()
}

fn expected_composition_descriptor_from_text_entry(
    text_entry: &BrowserFormTextEntry,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> BrowserCompositionInteractionDescriptor {
    let event_handlers = matching_interactive
        .map(|element| element.event_handlers.as_slice())
        .unwrap_or(&[]);
    let composition_handlers =
        expected_event_handlers_by_kind(event_handlers, expected_composition_event);
    let composition_start_handlers =
        expected_event_handlers_by_kind(event_handlers, expected_composition_start_event);
    let composition_update_handlers =
        expected_event_handlers_by_kind(event_handlers, expected_composition_update_event);
    let composition_end_handlers =
        expected_event_handlers_by_kind(event_handlers, expected_composition_end_event);
    let beforeinput_handlers =
        expected_event_handlers_by_kind(event_handlers, expected_beforeinput_event);
    let input_handlers = expected_event_handlers_by_kind(event_handlers, expected_text_input_event);
    let composition_block_reasons =
        expected_composition_block_reasons_for_text_entry(text_entry, matching_interactive);

    BrowserCompositionInteractionDescriptor {
        element: if text_entry.control_type == "textarea" {
            "textarea".to_string()
        } else {
            "input".to_string()
        },
        id: text_entry.id.clone(),
        role: Some("control".to_string()),
        authored_role: matching_interactive.and_then(|element| element.authored_role.clone()),
        source: "text-entry".to_string(),
        composition_kind: expected_composition_kind(
            text_entry.readonly,
            text_entry.control_type == "textarea",
            false,
            &composition_handlers,
            &beforeinput_handlers,
            &input_handlers,
            &composition_block_reasons,
        ),
        text: text_entry.text.clone(),
        accessible_name: text_entry.accessible_name.clone(),
        control_type: Some(text_entry.control_type.clone()),
        name: text_entry.name.clone(),
        form_owner: text_entry.form_owner.clone(),
        value: text_entry.value.clone(),
        contenteditable: None,
        editing_mode: (text_entry.control_type == "textarea").then(|| "plaintext".to_string()),
        spellcheck: text_entry.spellcheck.clone(),
        inputmode: text_entry.inputmode.clone(),
        enterkeyhint: text_entry.enterkeyhint.clone(),
        handler_count: composition_handlers.len() + beforeinput_handlers.len(),
        composition_handlers,
        composition_start_handlers,
        composition_update_handlers,
        composition_end_handlers,
        beforeinput_handlers,
        input_handlers,
        focusable: matching_interactive
            .and_then(|element| element.focusable)
            .unwrap_or(!text_entry.disabled),
        readonly: text_entry.readonly,
        disabled: text_entry.disabled,
        hidden: matching_interactive
            .map(|element| element.hidden)
            .unwrap_or(false),
        inert: matching_interactive
            .map(|element| element.inert)
            .unwrap_or(false),
        aria_hidden: matching_interactive
            .map(|element| element.aria_hidden)
            .unwrap_or(false),
        composition_blocked: !composition_block_reasons.is_empty(),
        composition_block_reasons,
    }
}

fn expected_composition_descriptor_from_interactive(
    element: &BrowserInteractiveElement,
) -> BrowserCompositionInteractionDescriptor {
    let composition_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_composition_event);
    let composition_start_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_composition_start_event);
    let composition_update_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_composition_update_event);
    let composition_end_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_composition_end_event);
    let beforeinput_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_beforeinput_event);
    let input_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_text_input_event);
    let composition_block_reasons = expected_composition_block_reasons_for_interactive(element);

    BrowserCompositionInteractionDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        source: "interactive".to_string(),
        composition_kind: expected_composition_kind(
            false,
            false,
            element.editing_mode.is_some(),
            &composition_handlers,
            &beforeinput_handlers,
            &input_handlers,
            &composition_block_reasons,
        ),
        text: element.text.clone(),
        accessible_name: element.accessible_name.clone(),
        control_type: None,
        name: None,
        form_owner: None,
        value: element.editing_mode.is_some().then(|| element.text.clone()),
        contenteditable: element.contenteditable.clone(),
        editing_mode: element.editing_mode.clone(),
        spellcheck: element.spellcheck.clone(),
        inputmode: None,
        enterkeyhint: None,
        handler_count: composition_handlers.len() + beforeinput_handlers.len(),
        composition_handlers,
        composition_start_handlers,
        composition_update_handlers,
        composition_end_handlers,
        beforeinput_handlers,
        input_handlers,
        focusable: element.focusable.unwrap_or(false),
        readonly: false,
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        composition_blocked: !composition_block_reasons.is_empty(),
        composition_block_reasons,
    }
}

fn expected_composition_descriptor_from_event(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserCompositionInteractionDescriptor {
    let composition_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_composition_event,
    );
    let composition_start_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_composition_start_event,
    );
    let composition_update_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_composition_update_event,
    );
    let composition_end_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_composition_end_event,
    );
    let beforeinput_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_beforeinput_event,
    );
    let input_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_text_input_event,
    );
    let composition_block_reasons = Vec::new();

    BrowserCompositionInteractionDescriptor {
        element: event_descriptor.element.clone(),
        id: event_descriptor.id.clone(),
        role: event_descriptor.role.clone(),
        authored_role: None,
        source: event_descriptor.source.clone(),
        composition_kind: expected_composition_kind(
            false,
            false,
            false,
            &composition_handlers,
            &beforeinput_handlers,
            &input_handlers,
            &composition_block_reasons,
        ),
        text: event_descriptor.text.clone(),
        accessible_name: None,
        control_type: None,
        name: None,
        form_owner: None,
        value: None,
        contenteditable: None,
        editing_mode: None,
        spellcheck: None,
        inputmode: None,
        enterkeyhint: None,
        handler_count: composition_handlers.len() + beforeinput_handlers.len(),
        composition_handlers,
        composition_start_handlers,
        composition_update_handlers,
        composition_end_handlers,
        beforeinput_handlers,
        input_handlers,
        focusable: false,
        readonly: false,
        disabled: false,
        hidden: false,
        inert: false,
        aria_hidden: false,
        composition_blocked: false,
        composition_block_reasons,
    }
}

fn expected_composition_block_reasons_for_text_entry(
    text_entry: &BrowserFormTextEntry,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if text_entry.disabled {
        reasons.push("disabled".to_string());
    }
    if text_entry.readonly {
        reasons.push("readonly".to_string());
    }
    if let Some(element) = matching_interactive {
        reasons.extend(expected_composition_block_reasons_for_interactive(element));
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn expected_composition_block_reasons_for_interactive(
    element: &BrowserInteractiveElement,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if element.disabled {
        reasons.push("disabled".to_string());
    }
    if element.hidden {
        reasons.push("hidden".to_string());
    }
    if element.inert {
        reasons.push("inert".to_string());
    }
    if element.aria_hidden {
        reasons.push("aria-hidden".to_string());
    }
    if element.aria_disabled.as_deref() == Some("true") {
        reasons.push("aria-disabled".to_string());
    }
    reasons
}

fn expected_composition_kind(
    readonly: bool,
    multiline: bool,
    editing_host: bool,
    composition_handlers: &[String],
    beforeinput_handlers: &[String],
    input_handlers: &[String],
    composition_block_reasons: &[String],
) -> String {
    if !composition_block_reasons.is_empty() {
        "blocked".to_string()
    } else if composition_handlers
        .iter()
        .any(|handler| handler == "oncompositionstart")
    {
        "ime-session".to_string()
    } else if composition_handlers
        .iter()
        .any(|handler| handler == "oncompositionupdate")
    {
        "ime-update".to_string()
    } else if composition_handlers
        .iter()
        .any(|handler| handler == "oncompositionend")
    {
        "ime-commit".to_string()
    } else if !beforeinput_handlers.is_empty() {
        "beforeinput-target".to_string()
    } else if editing_host {
        "editing-host".to_string()
    } else if readonly {
        "readonly-text".to_string()
    } else if multiline {
        "multiline-text".to_string()
    } else if !input_handlers.is_empty() {
        "input-target".to_string()
    } else {
        "text-control".to_string()
    }
}

fn expected_pointer_interaction_descriptors(
    interactive_elements: &[BrowserInteractiveElement],
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserPointerInteractionDescriptor> {
    let mut descriptors = Vec::new();

    for element in interactive_elements {
        if expected_interactive_has_pointer_state(element) {
            descriptors.push(expected_pointer_descriptor_from_interactive(element));
        }
    }

    for event_descriptor in event_handler_descriptors {
        if event_descriptor.source != "element" {
            continue;
        }
        if descriptors.iter().any(|descriptor| {
            descriptor.element == event_descriptor.element && descriptor.id == event_descriptor.id
        }) {
            continue;
        }
        if expected_event_descriptor_has_pointer_state(event_descriptor) {
            descriptors.push(expected_pointer_descriptor_from_event(event_descriptor));
        }
    }

    descriptors
}

fn expected_interactive_has_pointer_state(element: &BrowserInteractiveElement) -> bool {
    element.draggable.is_some()
        || element.command.is_some()
        || element.command_for.is_some()
        || element.popover_target.is_some()
        || element.popover_target_action.is_some()
        || !expected_event_handlers_by_kind(
            &element.event_handlers,
            expected_pointer_interaction_event,
        )
        .is_empty()
}

fn expected_event_descriptor_has_pointer_state(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_pointer_interaction_event,
    )
    .is_empty()
}

fn expected_pointer_descriptor_from_interactive(
    element: &BrowserInteractiveElement,
) -> BrowserPointerInteractionDescriptor {
    let pointer_handlers = expected_event_handlers_by_kind(
        &element.event_handlers,
        expected_pointer_interaction_event,
    );
    let mouse_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_mouse_event);
    let touch_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_touch_event);
    let wheel_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_wheel_event);
    let click_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_click_event);
    let drag_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_drag_event);
    let drop_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_drop_event);
    let pointer_block_reasons = expected_pointer_block_reasons_for_interactive(element);

    BrowserPointerInteractionDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        pointer_kind: expected_pointer_kind(
            element.draggable_state.as_deref(),
            element.command.is_some()
                || element.command_for.is_some()
                || element.popover_target.is_some()
                || element.popover_target_action.is_some(),
            element.editing_mode.is_some(),
            &pointer_handlers,
            &mouse_handlers,
            &touch_handlers,
            &wheel_handlers,
            &click_handlers,
            &drag_handlers,
            &drop_handlers,
            &pointer_block_reasons,
        ),
        text: element.text.clone(),
        accessible_name: element.accessible_name.clone(),
        control_type: (element.element == "button").then(|| "submit".to_string()),
        command: element.command.clone(),
        command_for: element.command_for.clone(),
        popover_target: element.popover_target.clone(),
        popover_target_action: element.popover_target_action.clone(),
        contenteditable: element.contenteditable.clone(),
        editing_mode: element.editing_mode.clone(),
        draggable: element.draggable.clone(),
        draggable_state: element.draggable_state.clone(),
        handler_count: pointer_handlers.len(),
        pointer_handlers,
        mouse_handlers,
        touch_handlers,
        wheel_handlers,
        click_handlers,
        drag_handlers,
        drop_handlers,
        focusable: element.focusable.unwrap_or(false),
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        pointer_blocked: !pointer_block_reasons.is_empty(),
        pointer_block_reasons,
    }
}

fn expected_pointer_descriptor_from_event(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserPointerInteractionDescriptor {
    let pointer_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_pointer_interaction_event,
    );
    let mouse_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_mouse_event);
    let touch_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_touch_event);
    let wheel_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_wheel_event);
    let click_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_click_event);
    let drag_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_drag_event);
    let drop_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_drop_event);
    let pointer_block_reasons = Vec::new();

    BrowserPointerInteractionDescriptor {
        element: event_descriptor.element.clone(),
        id: event_descriptor.id.clone(),
        role: event_descriptor.role.clone(),
        authored_role: None,
        pointer_kind: expected_pointer_kind(
            None,
            false,
            false,
            &pointer_handlers,
            &mouse_handlers,
            &touch_handlers,
            &wheel_handlers,
            &click_handlers,
            &drag_handlers,
            &drop_handlers,
            &pointer_block_reasons,
        ),
        text: event_descriptor.text.clone(),
        accessible_name: None,
        control_type: None,
        command: None,
        command_for: None,
        popover_target: None,
        popover_target_action: None,
        contenteditable: None,
        editing_mode: None,
        draggable: None,
        draggable_state: None,
        handler_count: pointer_handlers.len(),
        pointer_handlers,
        mouse_handlers,
        touch_handlers,
        wheel_handlers,
        click_handlers,
        drag_handlers,
        drop_handlers,
        focusable: false,
        disabled: false,
        hidden: false,
        inert: false,
        aria_hidden: false,
        pointer_blocked: false,
        pointer_block_reasons,
    }
}

fn expected_pointer_block_reasons_for_interactive(
    element: &BrowserInteractiveElement,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if element.disabled {
        reasons.push("disabled".to_string());
    }
    if element.hidden {
        reasons.push("hidden".to_string());
    }
    if element.inert {
        reasons.push("inert".to_string());
    }
    if element.aria_hidden {
        reasons.push("aria-hidden".to_string());
    }
    if element.aria_disabled.as_deref() == Some("true") {
        reasons.push("aria-disabled".to_string());
    }
    reasons
}

fn expected_pointer_kind(
    draggable_state: Option<&str>,
    command_target: bool,
    editing_host: bool,
    pointer_handlers: &[String],
    mouse_handlers: &[String],
    touch_handlers: &[String],
    wheel_handlers: &[String],
    click_handlers: &[String],
    drag_handlers: &[String],
    drop_handlers: &[String],
    pointer_block_reasons: &[String],
) -> String {
    if !pointer_block_reasons.is_empty() {
        "blocked".to_string()
    } else if expected_draggable_source(draggable_state) && !drop_handlers.is_empty() {
        "drag-source-and-drop-target".to_string()
    } else if expected_draggable_source(draggable_state) || !drag_handlers.is_empty() {
        "drag-source".to_string()
    } else if !drop_handlers.is_empty() {
        "drop-target".to_string()
    } else if !wheel_handlers.is_empty() {
        "wheel-target".to_string()
    } else if !touch_handlers.is_empty() {
        "touch-target".to_string()
    } else if pointer_handlers
        .iter()
        .any(|handler| handler.starts_with("onpointer"))
    {
        "pointer-target".to_string()
    } else if !mouse_handlers.is_empty() {
        "mouse-target".to_string()
    } else if !click_handlers.is_empty() {
        "click-target".to_string()
    } else if command_target {
        "command-target".to_string()
    } else if editing_host {
        "editing-target".to_string()
    } else {
        "metadata".to_string()
    }
}

fn expected_scroll_interaction_descriptors(
    aria_ranges: &[BrowserAriaRange],
    interactive_elements: &[BrowserInteractiveElement],
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserScrollInteractionDescriptor> {
    let mut descriptors = Vec::new();

    for range in aria_ranges {
        if range.role != "scrollbar" {
            continue;
        }
        let matching_interactive = range.id.as_deref().and_then(|id| {
            interactive_elements
                .iter()
                .find(|element| element.id.as_deref() == Some(id))
        });
        let event_descriptor = expected_matching_event_descriptor(
            event_handler_descriptors,
            &range.element,
            range.id.as_deref(),
        );
        descriptors.push(expected_scroll_descriptor_from_aria_range(
            range,
            matching_interactive,
            event_descriptor,
        ));
    }

    for element in interactive_elements {
        if descriptors
            .iter()
            .any(|descriptor| descriptor.element == element.element && descriptor.id == element.id)
        {
            continue;
        }
        if expected_interactive_has_scroll_state(element) {
            descriptors.push(expected_scroll_descriptor_from_interactive(element));
        }
    }

    for event_descriptor in event_handler_descriptors {
        if descriptors.iter().any(|descriptor| {
            descriptor.element == event_descriptor.element
                && descriptor.id == event_descriptor.id
                && (event_descriptor.source == "element"
                    || descriptor.source == event_descriptor.source)
        }) {
            continue;
        }
        if expected_event_descriptor_has_scroll_state(event_descriptor) {
            descriptors.push(expected_scroll_descriptor_from_event(event_descriptor));
        }
    }

    descriptors
}

fn expected_interactive_has_scroll_state(element: &BrowserInteractiveElement) -> bool {
    element.authored_role.as_deref() == Some("scrollbar")
        || !expected_event_handlers_by_kind(
            &element.event_handlers,
            expected_scroll_interaction_event,
        )
        .is_empty()
}

fn expected_event_descriptor_has_scroll_state(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_scroll_interaction_event,
    )
    .is_empty()
}

fn expected_scroll_descriptor_from_aria_range(
    range: &BrowserAriaRange,
    matching_interactive: Option<&BrowserInteractiveElement>,
    event_descriptor: Option<&BrowserEventHandlerDescriptor>,
) -> BrowserScrollInteractionDescriptor {
    let event_handlers = event_descriptor
        .map(|descriptor| descriptor.event_handlers.as_slice())
        .unwrap_or(&[]);
    let scroll_handlers = expected_event_handlers_by_kind(event_handlers, expected_scroll_event);
    let wheel_handlers = expected_event_handlers_by_kind(event_handlers, expected_wheel_event);
    let touch_handlers = expected_event_handlers_by_kind(event_handlers, expected_touch_event);
    let scroll_block_reasons = expected_scroll_block_reasons_for_range(range, matching_interactive);

    BrowserScrollInteractionDescriptor {
        element: range.element.clone(),
        id: range.id.clone(),
        role: Some(range.role.clone()),
        authored_role: Some(range.role.clone()),
        source: "aria-range".to_string(),
        scroll_kind: expected_scroll_kind(
            "aria-range",
            Some(range.role.as_str()),
            &scroll_handlers,
            &wheel_handlers,
            &touch_handlers,
            &scroll_block_reasons,
        ),
        text: range.text.clone(),
        accessible_name: range.accessible_name.clone(),
        accessible_description: range.accessible_description.clone(),
        aria_valuenow: range.aria_valuenow.clone(),
        aria_valuemin: range.aria_valuemin.clone(),
        aria_valuemax: range.aria_valuemax.clone(),
        aria_valuetext: range.aria_valuetext.clone(),
        aria_orientation: range.aria_orientation.clone(),
        aria_disabled: range.aria_disabled.clone(),
        aria_readonly: range.aria_readonly.clone(),
        tabindex: range.tabindex.clone(),
        handler_count: scroll_handlers.len() + wheel_handlers.len() + touch_handlers.len(),
        scroll_handlers,
        wheel_handlers,
        touch_handlers,
        focusable: matching_interactive
            .and_then(|element| element.focusable)
            .unwrap_or(false),
        disabled: matching_interactive
            .map(|element| element.disabled)
            .unwrap_or(false),
        hidden: matching_interactive
            .map(|element| element.hidden)
            .unwrap_or(false),
        inert: matching_interactive
            .map(|element| element.inert)
            .unwrap_or(false),
        aria_hidden: matching_interactive
            .map(|element| element.aria_hidden)
            .unwrap_or(false),
        scroll_blocked: !scroll_block_reasons.is_empty(),
        scroll_block_reasons,
    }
}

fn expected_scroll_descriptor_from_interactive(
    element: &BrowserInteractiveElement,
) -> BrowserScrollInteractionDescriptor {
    let scroll_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_scroll_event);
    let wheel_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_wheel_event);
    let touch_handlers =
        expected_event_handlers_by_kind(&element.event_handlers, expected_touch_event);
    let scroll_block_reasons = expected_scroll_block_reasons_for_interactive(element);

    BrowserScrollInteractionDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        source: "interactive".to_string(),
        scroll_kind: expected_scroll_kind(
            "interactive",
            element.authored_role.as_deref().or(element.role.as_deref()),
            &scroll_handlers,
            &wheel_handlers,
            &touch_handlers,
            &scroll_block_reasons,
        ),
        text: element.text.clone(),
        accessible_name: element.accessible_name.clone(),
        accessible_description: element.accessible_description.clone(),
        aria_valuenow: None,
        aria_valuemin: None,
        aria_valuemax: None,
        aria_valuetext: None,
        aria_orientation: None,
        aria_disabled: element.aria_disabled.clone(),
        aria_readonly: None,
        tabindex: element.tabindex.clone(),
        handler_count: scroll_handlers.len() + wheel_handlers.len() + touch_handlers.len(),
        scroll_handlers,
        wheel_handlers,
        touch_handlers,
        focusable: element.focusable.unwrap_or(false),
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        scroll_blocked: !scroll_block_reasons.is_empty(),
        scroll_block_reasons,
    }
}

fn expected_scroll_descriptor_from_event(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserScrollInteractionDescriptor {
    let scroll_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_scroll_event);
    let wheel_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_wheel_event);
    let touch_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_touch_event);
    let scroll_block_reasons = Vec::new();

    BrowserScrollInteractionDescriptor {
        element: event_descriptor.element.clone(),
        id: event_descriptor.id.clone(),
        role: event_descriptor.role.clone(),
        authored_role: None,
        source: event_descriptor.source.clone(),
        scroll_kind: expected_scroll_kind(
            event_descriptor.source.as_str(),
            event_descriptor.role.as_deref(),
            &scroll_handlers,
            &wheel_handlers,
            &touch_handlers,
            &scroll_block_reasons,
        ),
        text: event_descriptor.text.clone(),
        accessible_name: None,
        accessible_description: None,
        aria_valuenow: None,
        aria_valuemin: None,
        aria_valuemax: None,
        aria_valuetext: None,
        aria_orientation: None,
        aria_disabled: None,
        aria_readonly: None,
        tabindex: None,
        handler_count: scroll_handlers.len() + wheel_handlers.len() + touch_handlers.len(),
        scroll_handlers,
        wheel_handlers,
        touch_handlers,
        focusable: false,
        disabled: false,
        hidden: false,
        inert: false,
        aria_hidden: false,
        scroll_blocked: false,
        scroll_block_reasons,
    }
}

fn expected_scroll_block_reasons_for_range(
    range: &BrowserAriaRange,
    matching_interactive: Option<&BrowserInteractiveElement>,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if range.aria_disabled.as_deref() == Some("true") {
        reasons.push("aria-disabled".to_string());
    }
    if let Some(element) = matching_interactive {
        reasons.extend(expected_scroll_block_reasons_for_interactive(element));
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn expected_scroll_block_reasons_for_interactive(
    element: &BrowserInteractiveElement,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if element.disabled {
        reasons.push("disabled".to_string());
    }
    if element.hidden {
        reasons.push("hidden".to_string());
    }
    if element.inert {
        reasons.push("inert".to_string());
    }
    if element.aria_hidden {
        reasons.push("aria-hidden".to_string());
    }
    if element.aria_disabled.as_deref() == Some("true") {
        reasons.push("aria-disabled".to_string());
    }
    reasons
}

fn expected_scroll_kind(
    source: &str,
    role: Option<&str>,
    scroll_handlers: &[String],
    wheel_handlers: &[String],
    touch_handlers: &[String],
    scroll_block_reasons: &[String],
) -> String {
    if !scroll_block_reasons.is_empty() {
        "blocked".to_string()
    } else if role == Some("scrollbar") {
        "scrollbar".to_string()
    } else if !scroll_handlers.is_empty() {
        "scroll-handler".to_string()
    } else if !wheel_handlers.is_empty() {
        "wheel-target".to_string()
    } else if !touch_handlers.is_empty() {
        "touch-scroll-target".to_string()
    } else if source == "document" {
        "document-scroll".to_string()
    } else if source == "body" {
        "body-scroll".to_string()
    } else {
        "metadata".to_string()
    }
}

fn expected_tabindex_order(tabindex: Option<&str>) -> Option<i32> {
    tabindex.and_then(|tabindex| tabindex.trim().parse::<i32>().ok())
}

fn expected_event_handlers_by_kind(
    event_handlers: &[String],
    predicate: fn(&str) -> bool,
) -> Vec<String> {
    event_handlers
        .iter()
        .filter(|handler| predicate(handler.as_str()))
        .cloned()
        .collect()
}

fn expected_lifecycle_event_descriptors(
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserLifecycleEventDescriptor> {
    event_handler_descriptors
        .iter()
        .filter(|descriptor| expected_event_descriptor_has_lifecycle_state(descriptor))
        .map(expected_lifecycle_event_descriptor)
        .collect()
}

fn expected_event_descriptor_has_lifecycle_state(
    descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !descriptor.lifecycle_handlers.is_empty() || !descriptor.error_handlers.is_empty()
}

fn expected_lifecycle_event_descriptor(
    descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserLifecycleEventDescriptor {
    let load_handlers =
        expected_event_handlers_by_kind(&descriptor.event_handlers, expected_load_lifecycle_event);
    let unload_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_unload_lifecycle_event,
    );
    let visibility_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_visibility_lifecycle_event,
    );
    let history_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_history_lifecycle_event,
    );
    let network_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_network_lifecycle_event,
    );

    BrowserLifecycleEventDescriptor {
        element: descriptor.element.clone(),
        id: descriptor.id.clone(),
        classes: descriptor.classes.clone(),
        role: descriptor.role.clone(),
        source: descriptor.source.clone(),
        lifecycle_kind: expected_lifecycle_kind(
            &load_handlers,
            &unload_handlers,
            &visibility_handlers,
            &history_handlers,
            &network_handlers,
            &descriptor.error_handlers,
        ),
        text: descriptor.text.clone(),
        event_handlers: descriptor.event_handlers.clone(),
        lifecycle_handlers: descriptor.lifecycle_handlers.clone(),
        load_handlers,
        unload_handlers,
        visibility_handlers,
        history_handlers,
        network_handlers,
        error_handlers: descriptor.error_handlers.clone(),
        handler_count: descriptor.lifecycle_handlers.len() + descriptor.error_handlers.len(),
        document_scope: descriptor.source == "document",
        body_scope: descriptor.source == "body",
        error_recovery: !descriptor.error_handlers.is_empty(),
    }
}

fn expected_lifecycle_kind(
    load_handlers: &[String],
    unload_handlers: &[String],
    visibility_handlers: &[String],
    history_handlers: &[String],
    network_handlers: &[String],
    error_handlers: &[String],
) -> String {
    if !error_handlers.is_empty()
        && (!load_handlers.is_empty()
            || !unload_handlers.is_empty()
            || !visibility_handlers.is_empty()
            || !history_handlers.is_empty()
            || !network_handlers.is_empty())
    {
        "lifecycle-error".to_string()
    } else if !error_handlers.is_empty() {
        "error-recovery".to_string()
    } else if !unload_handlers.is_empty() {
        "unload".to_string()
    } else if !load_handlers.is_empty() {
        "load".to_string()
    } else if !visibility_handlers.is_empty() {
        "visibility".to_string()
    } else if !history_handlers.is_empty() {
        "history".to_string()
    } else if !network_handlers.is_empty() {
        "network".to_string()
    } else {
        "lifecycle".to_string()
    }
}

fn expected_animation_interaction_descriptors(
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserAnimationInteractionDescriptor> {
    event_handler_descriptors
        .iter()
        .filter(|descriptor| expected_event_descriptor_has_animation_interaction_state(descriptor))
        .map(expected_animation_interaction_descriptor)
        .collect()
}

fn expected_event_descriptor_has_animation_interaction_state(
    descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_animation_interaction_event,
    )
    .is_empty()
}

fn expected_animation_interaction_descriptor(
    descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserAnimationInteractionDescriptor {
    let animation_handlers =
        expected_event_handlers_by_kind(&descriptor.event_handlers, expected_animation_event);
    let animation_start_handlers =
        expected_event_handlers_by_kind(&descriptor.event_handlers, expected_animation_start_event);
    let animation_iteration_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_animation_iteration_event,
    );
    let animation_end_handlers =
        expected_event_handlers_by_kind(&descriptor.event_handlers, expected_animation_end_event);
    let animation_cancel_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_animation_cancel_event,
    );
    let transition_handlers =
        expected_event_handlers_by_kind(&descriptor.event_handlers, expected_transition_event);
    let transition_run_handlers =
        expected_event_handlers_by_kind(&descriptor.event_handlers, expected_transition_run_event);
    let transition_start_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_transition_start_event,
    );
    let transition_end_handlers =
        expected_event_handlers_by_kind(&descriptor.event_handlers, expected_transition_end_event);
    let transition_cancel_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_transition_cancel_event,
    );

    BrowserAnimationInteractionDescriptor {
        element: descriptor.element.clone(),
        id: descriptor.id.clone(),
        classes: descriptor.classes.clone(),
        role: descriptor.role.clone(),
        source: descriptor.source.clone(),
        animation_kind: expected_animation_interaction_kind(
            &animation_handlers,
            &animation_start_handlers,
            &animation_iteration_handlers,
            &animation_end_handlers,
            &animation_cancel_handlers,
            &transition_handlers,
            &transition_run_handlers,
            &transition_start_handlers,
            &transition_end_handlers,
            &transition_cancel_handlers,
        ),
        text: descriptor.text.clone(),
        event_handlers: descriptor.event_handlers.clone(),
        animation_handlers,
        animation_start_handlers,
        animation_iteration_handlers,
        animation_end_handlers,
        animation_cancel_handlers,
        transition_handlers,
        transition_run_handlers,
        transition_start_handlers,
        transition_end_handlers,
        transition_cancel_handlers,
        handler_count: descriptor
            .event_handlers
            .iter()
            .filter(|handler| expected_animation_interaction_event(handler.as_str()))
            .count(),
        document_scope: descriptor.source == "document",
        body_scope: descriptor.source == "body",
    }
}

fn expected_animation_interaction_kind(
    animation_handlers: &[String],
    animation_start_handlers: &[String],
    animation_iteration_handlers: &[String],
    animation_end_handlers: &[String],
    animation_cancel_handlers: &[String],
    transition_handlers: &[String],
    transition_run_handlers: &[String],
    transition_start_handlers: &[String],
    transition_end_handlers: &[String],
    transition_cancel_handlers: &[String],
) -> String {
    if !animation_cancel_handlers.is_empty() || !transition_cancel_handlers.is_empty() {
        "animation-cancel".to_string()
    } else if !animation_handlers.is_empty() && !transition_handlers.is_empty() {
        "animation-transition".to_string()
    } else if !animation_iteration_handlers.is_empty() {
        "animation-iteration".to_string()
    } else if !animation_end_handlers.is_empty() {
        "animation-end".to_string()
    } else if !animation_start_handlers.is_empty() {
        "animation-start".to_string()
    } else if !transition_end_handlers.is_empty() {
        "transition-end".to_string()
    } else if !transition_start_handlers.is_empty() {
        "transition-start".to_string()
    } else if !transition_run_handlers.is_empty() {
        "transition-run".to_string()
    } else if !animation_handlers.is_empty() {
        "animation".to_string()
    } else {
        "transition".to_string()
    }
}

fn expected_fullscreen_interaction_descriptors(
    embedded_policy_descriptors: &[BrowserEmbeddedPolicyDescriptor],
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserFullscreenInteractionDescriptor> {
    let mut descriptors = Vec::new();

    for policy in embedded_policy_descriptors {
        if !policy.allowfullscreen && !expected_allow_fullscreen_policy(policy.allow.as_deref()) {
            continue;
        }

        let matching_event_descriptor = event_handler_descriptors.iter().find(|event_descriptor| {
            expected_fullscreen_descriptor_matches_policy(event_descriptor, policy)
        });
        descriptors.push(expected_fullscreen_descriptor_from_embedded_policy(
            policy,
            matching_event_descriptor,
        ));
    }

    for event_descriptor in event_handler_descriptors {
        if !expected_event_descriptor_has_fullscreen_interaction_state(event_descriptor) {
            continue;
        }
        if descriptors.iter().any(|descriptor| {
            expected_fullscreen_descriptor_matches_event(descriptor, event_descriptor)
        }) {
            continue;
        }
        descriptors.push(expected_fullscreen_descriptor_from_event(event_descriptor));
    }

    descriptors
}

fn expected_fullscreen_descriptor_matches_policy(
    event_descriptor: &BrowserEventHandlerDescriptor,
    policy: &BrowserEmbeddedPolicyDescriptor,
) -> bool {
    event_descriptor.element == policy.element
        && event_descriptor.id.as_deref() == policy.browsing_context_name.as_deref()
}

fn expected_fullscreen_descriptor_matches_event(
    descriptor: &BrowserFullscreenInteractionDescriptor,
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    descriptor.element == event_descriptor.element && descriptor.id == event_descriptor.id
}

fn expected_event_descriptor_has_fullscreen_interaction_state(
    descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !expected_event_handlers_by_kind(&descriptor.event_handlers, expected_fullscreen_event)
        .is_empty()
}

fn expected_fullscreen_descriptor_from_embedded_policy(
    policy: &BrowserEmbeddedPolicyDescriptor,
    event_descriptor: Option<&BrowserEventHandlerDescriptor>,
) -> BrowserFullscreenInteractionDescriptor {
    let event_handlers = event_descriptor
        .map(|descriptor| descriptor.event_handlers.clone())
        .unwrap_or_default();
    let fullscreen_handlers =
        expected_event_handlers_by_kind(&event_handlers, expected_fullscreen_event);
    let fullscreen_change_handlers =
        expected_event_handlers_by_kind(&event_handlers, expected_fullscreen_change_event);
    let fullscreen_error_handlers =
        expected_event_handlers_by_kind(&event_handlers, expected_fullscreen_error_event);
    let fullscreen_allowed =
        policy.allowfullscreen || expected_allow_fullscreen_policy(policy.allow.as_deref());

    BrowserFullscreenInteractionDescriptor {
        element: policy.element.clone(),
        id: policy.browsing_context_name.clone(),
        classes: event_descriptor
            .map(|descriptor| descriptor.classes.clone())
            .unwrap_or_default(),
        role: event_descriptor.and_then(|descriptor| descriptor.role.clone()),
        source: "embedded-policy".to_string(),
        fullscreen_kind: expected_fullscreen_interaction_kind(
            fullscreen_allowed,
            &fullscreen_change_handlers,
            &fullscreen_error_handlers,
        ),
        text: policy.fallback_text.clone(),
        event_handlers,
        handler_count: fullscreen_handlers.len(),
        fullscreen_handlers,
        fullscreen_change_handlers,
        fullscreen_error_handlers,
        allow: policy.allow.clone(),
        allow_tokens: expected_permission_policy_tokens(policy.allow.as_deref()),
        allowfullscreen: policy.allowfullscreen,
        fullscreen_allowed,
        embedded_context: true,
        document_scope: false,
        body_scope: false,
    }
}

fn expected_fullscreen_descriptor_from_event(
    descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserFullscreenInteractionDescriptor {
    let fullscreen_handlers =
        expected_event_handlers_by_kind(&descriptor.event_handlers, expected_fullscreen_event);
    let fullscreen_change_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_fullscreen_change_event,
    );
    let fullscreen_error_handlers = expected_event_handlers_by_kind(
        &descriptor.event_handlers,
        expected_fullscreen_error_event,
    );

    BrowserFullscreenInteractionDescriptor {
        element: descriptor.element.clone(),
        id: descriptor.id.clone(),
        classes: descriptor.classes.clone(),
        role: descriptor.role.clone(),
        source: descriptor.source.clone(),
        fullscreen_kind: expected_fullscreen_interaction_kind(
            false,
            &fullscreen_change_handlers,
            &fullscreen_error_handlers,
        ),
        text: descriptor.text.clone(),
        event_handlers: descriptor.event_handlers.clone(),
        handler_count: fullscreen_handlers.len(),
        fullscreen_handlers,
        fullscreen_change_handlers,
        fullscreen_error_handlers,
        allow: None,
        allow_tokens: Vec::new(),
        allowfullscreen: false,
        fullscreen_allowed: false,
        embedded_context: false,
        document_scope: descriptor.source == "document",
        body_scope: descriptor.source == "body",
    }
}

fn expected_fullscreen_interaction_kind(
    fullscreen_allowed: bool,
    fullscreen_change_handlers: &[String],
    fullscreen_error_handlers: &[String],
) -> String {
    if fullscreen_allowed && !fullscreen_error_handlers.is_empty() {
        "fullscreen-policy-error".to_string()
    } else if fullscreen_allowed && !fullscreen_change_handlers.is_empty() {
        "fullscreen-policy-change".to_string()
    } else if fullscreen_allowed {
        "fullscreen-enabled".to_string()
    } else if !fullscreen_error_handlers.is_empty() {
        "fullscreen-error".to_string()
    } else {
        "fullscreen-change".to_string()
    }
}

fn expected_allow_fullscreen_policy(allow: Option<&str>) -> bool {
    allow
        .into_iter()
        .flat_map(|allow| allow.split(';'))
        .filter_map(expected_permission_policy_feature)
        .any(|feature| feature == "fullscreen")
}

fn expected_permission_policy_tokens(allow: Option<&str>) -> Vec<String> {
    allow
        .into_iter()
        .flat_map(|allow| allow.split(';'))
        .filter_map(expected_permission_policy_feature)
        .collect()
}

fn expected_permission_policy_feature(directive: &str) -> Option<String> {
    directive
        .split_whitespace()
        .next()
        .map(str::to_ascii_lowercase)
        .filter(|feature| !feature.is_empty())
}

fn expected_context_menu_interaction_descriptors(
    interactive_elements: &[BrowserInteractiveElement],
    event_handler_descriptors: &[BrowserEventHandlerDescriptor],
) -> Vec<BrowserContextMenuInteractionDescriptor> {
    let mut descriptors = Vec::new();

    for element in interactive_elements {
        let event_descriptor = expected_matching_event_descriptor(
            event_handler_descriptors,
            &element.element,
            element.id.as_deref(),
        );
        if expected_interactive_has_context_menu_state(element, event_descriptor) {
            descriptors.push(expected_context_menu_descriptor_from_interactive(
                element,
                event_descriptor,
            ));
        }
    }

    for event_descriptor in event_handler_descriptors {
        if event_descriptor.source != "element" {
            continue;
        }
        if descriptors.iter().any(|descriptor| {
            descriptor.element == event_descriptor.element && descriptor.id == event_descriptor.id
        }) {
            continue;
        }
        if expected_event_descriptor_has_context_menu_state(event_descriptor) {
            descriptors.push(expected_context_menu_descriptor_from_event(
                event_descriptor,
            ));
        }
    }

    descriptors
}

fn expected_interactive_has_context_menu_state(
    element: &BrowserInteractiveElement,
    event_descriptor: Option<&BrowserEventHandlerDescriptor>,
) -> bool {
    expected_aria_haspopup_menu(element.aria_haspopup.as_deref())
        || element
            .authored_role
            .as_deref()
            .is_some_and(expected_menu_role)
        || (element.popover.is_some()
            && element
                .authored_role
                .as_deref()
                .is_some_and(expected_menu_role))
        || (element.popover_target.is_some()
            && expected_aria_haspopup_menu(element.aria_haspopup.as_deref()))
        || event_descriptor
            .map(expected_event_descriptor_has_context_menu_state)
            .unwrap_or(false)
}

fn expected_event_descriptor_has_context_menu_state(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> bool {
    !expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_context_menu_event,
    )
    .is_empty()
}

fn expected_context_menu_descriptor_from_interactive(
    element: &BrowserInteractiveElement,
    event_descriptor: Option<&BrowserEventHandlerDescriptor>,
) -> BrowserContextMenuInteractionDescriptor {
    let event_handlers = event_descriptor
        .map(|descriptor| descriptor.event_handlers.clone())
        .unwrap_or_else(|| element.event_handlers.clone());
    let contextmenu_handlers =
        expected_event_handlers_by_kind(&event_handlers, expected_context_menu_event);
    let pointer_handlers =
        expected_event_handlers_by_kind(&event_handlers, expected_pointer_interaction_event);
    let keyboard_handlers =
        expected_event_handlers_by_kind(&event_handlers, expected_keyboard_event);
    let handler_count = contextmenu_handlers.len() + keyboard_handlers.len();
    let context_menu_block_reasons = expected_pointer_block_reasons_for_interactive(element);
    let menu_role = element
        .authored_role
        .as_deref()
        .is_some_and(expected_menu_role);
    let menu_invoker = expected_aria_haspopup_menu(element.aria_haspopup.as_deref())
        || (element.popover_target.is_some()
            && expected_aria_haspopup_menu(element.aria_haspopup.as_deref()));

    BrowserContextMenuInteractionDescriptor {
        element: element.element.clone(),
        id: element.id.clone(),
        role: element.role.clone(),
        authored_role: element.authored_role.clone(),
        source: "interactive".to_string(),
        context_menu_kind: expected_context_menu_kind(
            menu_invoker,
            menu_role,
            element.popover.is_some() && menu_role,
            &contextmenu_handlers,
            &keyboard_handlers,
            &context_menu_block_reasons,
        ),
        text: element.text.clone(),
        accessible_name: element.accessible_name.clone(),
        accessible_description: element.accessible_description.clone(),
        aria_haspopup: element.aria_haspopup.clone(),
        aria_controls: element.aria_controls.clone(),
        aria_expanded: element.aria_expanded.clone(),
        popover: element.popover.clone(),
        popover_target: element.popover_target.clone(),
        popover_target_action: element.popover_target_action.clone(),
        command: element.command.clone(),
        command_for: element.command_for.clone(),
        event_handlers,
        contextmenu_handlers,
        pointer_handlers,
        keyboard_handlers,
        handler_count,
        focusable: element.focusable.unwrap_or(false),
        disabled: element.disabled,
        hidden: element.hidden,
        inert: element.inert,
        aria_hidden: element.aria_hidden,
        context_menu_blocked: !context_menu_block_reasons.is_empty(),
        context_menu_block_reasons,
    }
}

fn expected_context_menu_descriptor_from_event(
    event_descriptor: &BrowserEventHandlerDescriptor,
) -> BrowserContextMenuInteractionDescriptor {
    let contextmenu_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_context_menu_event,
    );
    let pointer_handlers = expected_event_handlers_by_kind(
        &event_descriptor.event_handlers,
        expected_pointer_interaction_event,
    );
    let keyboard_handlers =
        expected_event_handlers_by_kind(&event_descriptor.event_handlers, expected_keyboard_event);
    let handler_count = contextmenu_handlers.len() + keyboard_handlers.len();
    let context_menu_block_reasons = Vec::new();

    BrowserContextMenuInteractionDescriptor {
        element: event_descriptor.element.clone(),
        id: event_descriptor.id.clone(),
        role: event_descriptor.role.clone(),
        authored_role: None,
        source: "event-handler".to_string(),
        context_menu_kind: expected_context_menu_kind(
            false,
            false,
            false,
            &contextmenu_handlers,
            &keyboard_handlers,
            &context_menu_block_reasons,
        ),
        text: event_descriptor.text.clone(),
        accessible_name: None,
        accessible_description: None,
        aria_haspopup: None,
        aria_controls: Vec::new(),
        aria_expanded: None,
        popover: None,
        popover_target: None,
        popover_target_action: None,
        command: None,
        command_for: None,
        event_handlers: event_descriptor.event_handlers.clone(),
        contextmenu_handlers,
        pointer_handlers,
        keyboard_handlers,
        handler_count,
        focusable: false,
        disabled: false,
        hidden: false,
        inert: false,
        aria_hidden: false,
        context_menu_blocked: false,
        context_menu_block_reasons,
    }
}

fn expected_context_menu_kind(
    menu_invoker: bool,
    menu_role: bool,
    popover_surface: bool,
    contextmenu_handlers: &[String],
    keyboard_handlers: &[String],
    context_menu_block_reasons: &[String],
) -> String {
    if !context_menu_block_reasons.is_empty() {
        "blocked".to_string()
    } else if !contextmenu_handlers.is_empty() && menu_invoker {
        "custom-menu-handler".to_string()
    } else if !contextmenu_handlers.is_empty() {
        "context-menu-handler".to_string()
    } else if menu_invoker {
        "menu-invoker".to_string()
    } else if popover_surface {
        "menu-surface".to_string()
    } else if menu_role {
        "menu-item".to_string()
    } else if !keyboard_handlers.is_empty() {
        "keyboard-menu".to_string()
    } else {
        "context-menu".to_string()
    }
}

fn expected_aria_haspopup_menu(value: Option<&str>) -> bool {
    matches!(value, Some("true" | "menu"))
}

fn expected_menu_role(role: &str) -> bool {
    matches!(
        role,
        "menu" | "menubar" | "menuitem" | "menuitemcheckbox" | "menuitemradio"
    )
}

fn expected_keyboard_event(handler: &str) -> bool {
    matches!(handler, "onkeydown" | "onkeypress" | "onkeyup")
}

fn expected_input_event(handler: &str) -> bool {
    matches!(
        handler,
        "onbeforeinput"
            | "oninput"
            | "onchange"
            | "onselect"
            | "oncompositionstart"
            | "oncompositionupdate"
            | "oncompositionend"
    )
}

fn expected_animation_interaction_event(handler: &str) -> bool {
    expected_animation_event(handler) || expected_transition_event(handler)
}

fn expected_animation_event(handler: &str) -> bool {
    matches!(
        handler,
        "onanimationstart" | "onanimationiteration" | "onanimationend" | "onanimationcancel"
    )
}

fn expected_animation_start_event(handler: &str) -> bool {
    handler == "onanimationstart"
}

fn expected_animation_iteration_event(handler: &str) -> bool {
    handler == "onanimationiteration"
}

fn expected_animation_end_event(handler: &str) -> bool {
    handler == "onanimationend"
}

fn expected_animation_cancel_event(handler: &str) -> bool {
    handler == "onanimationcancel"
}

fn expected_transition_event(handler: &str) -> bool {
    matches!(
        handler,
        "ontransitionrun" | "ontransitionstart" | "ontransitionend" | "ontransitioncancel"
    )
}

fn expected_transition_run_event(handler: &str) -> bool {
    handler == "ontransitionrun"
}

fn expected_transition_start_event(handler: &str) -> bool {
    handler == "ontransitionstart"
}

fn expected_transition_end_event(handler: &str) -> bool {
    handler == "ontransitionend"
}

fn expected_transition_cancel_event(handler: &str) -> bool {
    handler == "ontransitioncancel"
}

fn expected_fullscreen_event(handler: &str) -> bool {
    matches!(handler, "onfullscreenchange" | "onfullscreenerror")
}

fn expected_fullscreen_change_event(handler: &str) -> bool {
    handler == "onfullscreenchange"
}

fn expected_fullscreen_error_event(handler: &str) -> bool {
    handler == "onfullscreenerror"
}

fn expected_load_lifecycle_event(handler: &str) -> bool {
    matches!(
        handler,
        "onload" | "onpageshow" | "onreadystatechange" | "ondomcontentloaded"
    )
}

fn expected_unload_lifecycle_event(handler: &str) -> bool {
    matches!(handler, "onunload" | "onbeforeunload" | "onpagehide")
}

fn expected_visibility_lifecycle_event(handler: &str) -> bool {
    handler == "onvisibilitychange"
}

fn expected_history_lifecycle_event(handler: &str) -> bool {
    matches!(handler, "onhashchange" | "onpopstate")
}

fn expected_network_lifecycle_event(handler: &str) -> bool {
    matches!(handler, "ononline" | "onoffline")
}

fn expected_pointer_event(handler: &str) -> bool {
    matches!(
        handler,
        "onpointerdown"
            | "onpointermove"
            | "onpointerup"
            | "onpointercancel"
            | "onpointerenter"
            | "onpointerleave"
            | "onpointerover"
            | "onpointerout"
            | "onmousedown"
            | "onmousemove"
            | "onmouseup"
            | "onmouseenter"
            | "onmouseleave"
            | "onmouseover"
            | "onmouseout"
            | "ontouchstart"
            | "ontouchmove"
            | "ontouchend"
            | "ontouchcancel"
            | "ondrag"
            | "ondragstart"
            | "ondragend"
            | "ondragenter"
            | "ondragleave"
            | "ondragover"
            | "ondrop"
            | "onwheel"
    )
}

fn expected_pointer_interaction_event(handler: &str) -> bool {
    expected_pointer_event(handler) || expected_click_event(handler)
}

fn expected_mouse_event(handler: &str) -> bool {
    matches!(
        handler,
        "onmousedown"
            | "onmousemove"
            | "onmouseup"
            | "onmouseenter"
            | "onmouseleave"
            | "onmouseover"
            | "onmouseout"
    )
}

fn expected_touch_event(handler: &str) -> bool {
    matches!(
        handler,
        "ontouchstart" | "ontouchmove" | "ontouchend" | "ontouchcancel"
    )
}

fn expected_wheel_event(handler: &str) -> bool {
    handler == "onwheel"
}

fn expected_scroll_event(handler: &str) -> bool {
    matches!(handler, "onscroll" | "onscrollend")
}

fn expected_scroll_interaction_event(handler: &str) -> bool {
    expected_scroll_event(handler) || expected_wheel_event(handler) || expected_touch_event(handler)
}

fn expected_click_event(handler: &str) -> bool {
    matches!(handler, "onclick" | "ondblclick" | "oncontextmenu")
}

fn expected_context_menu_event(handler: &str) -> bool {
    handler == "oncontextmenu"
}

fn expected_drag_event(handler: &str) -> bool {
    matches!(handler, "ondrag" | "ondragstart" | "ondragend")
}

fn expected_drop_event(handler: &str) -> bool {
    matches!(
        handler,
        "ondragenter" | "ondragleave" | "ondragover" | "ondrop"
    )
}

fn expected_clipboard_event(handler: &str) -> bool {
    matches!(handler, "oncopy" | "oncut" | "onpaste")
}

fn expected_copy_event(handler: &str) -> bool {
    handler == "oncopy"
}

fn expected_cut_event(handler: &str) -> bool {
    handler == "oncut"
}

fn expected_paste_event(handler: &str) -> bool {
    handler == "onpaste"
}

fn expected_selection_event(handler: &str) -> bool {
    matches!(handler, "onselect" | "onselectionchange")
}

fn expected_select_event(handler: &str) -> bool {
    handler == "onselect"
}

fn expected_selection_change_event(handler: &str) -> bool {
    handler == "onselectionchange"
}

fn expected_selection_input_event(handler: &str) -> bool {
    matches!(
        handler,
        "onbeforeinput"
            | "oninput"
            | "onchange"
            | "oncompositionstart"
            | "oncompositionupdate"
            | "oncompositionend"
    )
}

fn expected_composition_event(handler: &str) -> bool {
    matches!(
        handler,
        "oncompositionstart" | "oncompositionupdate" | "oncompositionend"
    )
}

fn expected_composition_start_event(handler: &str) -> bool {
    handler == "oncompositionstart"
}

fn expected_composition_update_event(handler: &str) -> bool {
    handler == "oncompositionupdate"
}

fn expected_composition_end_event(handler: &str) -> bool {
    handler == "oncompositionend"
}

fn expected_beforeinput_event(handler: &str) -> bool {
    handler == "onbeforeinput"
}

fn expected_text_input_event(handler: &str) -> bool {
    matches!(handler, "onbeforeinput" | "oninput")
}

fn expected_script_execution_descriptors(
    scripts: &[BrowserScript],
) -> Vec<BrowserScriptExecutionDescriptor> {
    scripts
        .iter()
        .map(expected_script_execution_descriptor)
        .collect()
}

fn expected_script_execution_descriptor(
    script: &BrowserScript,
) -> BrowserScriptExecutionDescriptor {
    let text_length = script
        .text
        .as_ref()
        .map(|text| text.chars().count())
        .unwrap_or_default();
    BrowserScriptExecutionDescriptor {
        script_kind: script.script_kind.clone(),
        execution_kind: if script.src.is_some() {
            "external".to_string()
        } else {
            "inline".to_string()
        },
        src: script.src.clone(),
        resolved_src: script.resolved_src.clone(),
        type_hint: script.type_hint.clone(),
        async_script: script.async_script,
        defer_script: script.defer_script,
        nomodule: script.nomodule,
        integrity: script.integrity.clone(),
        crossorigin: script.crossorigin.clone(),
        nonce: script.nonce.clone(),
        referrerpolicy: script.referrerpolicy.clone(),
        fetchpriority: script.fetchpriority.clone(),
        blocking: script.blocking.clone(),
        blocking_tokens: script.blocking_tokens.clone(),
        blocking_token_count: script.blocking_tokens.len(),
        render_blocking: script
            .blocking_tokens
            .iter()
            .any(|token| token.eq_ignore_ascii_case("render")),
        has_text: script.text.is_some(),
        text_length,
    }
}

fn expected_script_storage_access_descriptors(
    scripts: &[BrowserScript],
) -> Vec<BrowserScriptStorageAccessDescriptor> {
    scripts
        .iter()
        .filter_map(expected_script_storage_access_descriptor)
        .collect()
}

fn expected_script_storage_access_descriptor(
    script: &BrowserScript,
) -> Option<BrowserScriptStorageAccessDescriptor> {
    let text = script.text.as_deref().unwrap_or_default();
    let normalized_text = text.to_ascii_lowercase();
    let uses_local_storage = normalized_text.contains("localstorage");
    let uses_session_storage = normalized_text.contains("sessionstorage");
    let uses_cookies =
        normalized_text.contains("document.cookie") || normalized_text.contains("cookie=");
    let uses_indexed_db = normalized_text.contains("indexeddb");
    let uses_cache_storage = normalized_text.contains("caches.")
        || normalized_text.contains("caches.open")
        || normalized_text.contains("cachestorage");
    let uses_service_worker = normalized_text.contains("serviceworker")
        || normalized_text.contains("service-worker")
        || normalized_text.contains("navigator.serviceworker");
    let uses_storage_manager = normalized_text.contains("navigator.storage")
        || normalized_text.contains(".persist(")
        || normalized_text.contains(".estimate(");
    let listens_storage_events = normalized_text.contains("addeventlistener('storage'")
        || normalized_text.contains("addeventlistener(\"storage\"")
        || normalized_text.contains("onstorage");
    let storage_targets = expected_script_storage_targets(
        uses_local_storage,
        uses_session_storage,
        uses_cookies,
        uses_indexed_db,
        uses_cache_storage,
        uses_service_worker,
        uses_storage_manager,
        listens_storage_events,
    );
    if storage_targets.is_empty() {
        return None;
    }

    let storage_block_reasons = expected_script_storage_block_reasons(script);
    Some(BrowserScriptStorageAccessDescriptor {
        script_kind: script.script_kind.clone(),
        access_kind: expected_script_storage_access_kind(
            uses_local_storage,
            uses_session_storage,
            uses_cookies,
            uses_indexed_db,
            uses_cache_storage,
            uses_service_worker,
            uses_storage_manager,
            listens_storage_events,
        ),
        src: script.src.clone(),
        resolved_src: script.resolved_src.clone(),
        type_hint: script.type_hint.clone(),
        execution_kind: if script.src.is_some() {
            "external".to_string()
        } else {
            "inline".to_string()
        },
        has_text: script.text.is_some(),
        text_length: text.chars().count(),
        storage_target_count: storage_targets.len(),
        storage_targets,
        uses_local_storage,
        uses_session_storage,
        uses_cookies,
        uses_indexed_db,
        uses_cache_storage,
        uses_service_worker,
        uses_storage_manager,
        listens_storage_events,
        storage_blocked: !storage_block_reasons.is_empty(),
        storage_block_reasons,
    })
}

fn expected_script_storage_targets(
    uses_local_storage: bool,
    uses_session_storage: bool,
    uses_cookies: bool,
    uses_indexed_db: bool,
    uses_cache_storage: bool,
    uses_service_worker: bool,
    uses_storage_manager: bool,
    listens_storage_events: bool,
) -> Vec<String> {
    let mut targets = Vec::new();
    if uses_local_storage {
        targets.push("localStorage".to_string());
    }
    if uses_session_storage {
        targets.push("sessionStorage".to_string());
    }
    if uses_cookies {
        targets.push("cookies".to_string());
    }
    if uses_indexed_db {
        targets.push("indexedDB".to_string());
    }
    if uses_cache_storage {
        targets.push("CacheStorage".to_string());
    }
    if uses_service_worker {
        targets.push("serviceWorker".to_string());
    }
    if uses_storage_manager {
        targets.push("StorageManager".to_string());
    }
    if listens_storage_events {
        targets.push("storage-event".to_string());
    }
    targets
}

fn expected_script_storage_access_kind(
    uses_local_storage: bool,
    uses_session_storage: bool,
    uses_cookies: bool,
    uses_indexed_db: bool,
    uses_cache_storage: bool,
    uses_service_worker: bool,
    uses_storage_manager: bool,
    listens_storage_events: bool,
) -> String {
    if uses_service_worker || uses_cache_storage {
        "worker-cache-storage".to_string()
    } else if uses_indexed_db {
        "database-storage".to_string()
    } else if uses_storage_manager {
        "storage-manager".to_string()
    } else if uses_local_storage || uses_session_storage || uses_cookies {
        "client-key-value-storage".to_string()
    } else if listens_storage_events {
        "storage-event-listener".to_string()
    } else {
        "storage-metadata".to_string()
    }
}

fn expected_script_storage_block_reasons(script: &BrowserScript) -> Vec<String> {
    let mut reasons = Vec::new();
    if script.script_kind == "data" {
        reasons.push("non-executable-script-type".to_string());
    }
    if script.nomodule {
        reasons.push("nomodule-fallback".to_string());
    }
    reasons
}

fn expected_script_worker_messaging_descriptors(
    scripts: &[BrowserScript],
) -> Vec<BrowserScriptWorkerMessagingDescriptor> {
    scripts
        .iter()
        .filter_map(expected_script_worker_messaging_descriptor)
        .collect()
}

fn expected_script_worker_messaging_descriptor(
    script: &BrowserScript,
) -> Option<BrowserScriptWorkerMessagingDescriptor> {
    let text = script.text.as_deref().unwrap_or_default();
    let normalized_text = text.to_ascii_lowercase();
    let creates_shared_worker = normalized_text.contains("new sharedworker");
    let creates_worker =
        normalized_text.contains("new worker") && !normalized_text.contains("new sharedworker");
    let registers_service_worker = normalized_text.contains("serviceworker.register")
        || normalized_text.contains("service-worker.register")
        || normalized_text.contains("navigator.serviceworker.register");
    let uses_post_message =
        normalized_text.contains("postmessage(") || normalized_text.contains(".postmessage(");
    let listens_message_events = normalized_text.contains("addeventlistener('message'")
        || normalized_text.contains("addeventlistener(\"message\"")
        || normalized_text.contains("onmessage");
    let uses_message_channel = normalized_text.contains("messagechannel");
    let uses_broadcast_channel = normalized_text.contains("broadcastchannel");
    let uses_import_scripts = normalized_text.contains("importscripts(");
    let module_worker_hint = normalized_text.contains("type:'module'")
        || normalized_text.contains("type: 'module'")
        || normalized_text.contains("type:\"module\"")
        || normalized_text.contains("type: \"module\"");
    let messaging_targets = expected_script_worker_messaging_targets(
        creates_worker,
        creates_shared_worker,
        registers_service_worker,
        uses_post_message,
        listens_message_events,
        uses_message_channel,
        uses_broadcast_channel,
        uses_import_scripts,
        module_worker_hint,
    );
    if messaging_targets.is_empty() {
        return None;
    }

    let messaging_block_reasons = expected_script_worker_messaging_block_reasons(script);
    Some(BrowserScriptWorkerMessagingDescriptor {
        script_kind: script.script_kind.clone(),
        messaging_kind: expected_script_worker_messaging_kind(
            creates_worker,
            creates_shared_worker,
            registers_service_worker,
            uses_post_message,
            listens_message_events,
            uses_message_channel,
            uses_broadcast_channel,
            uses_import_scripts,
            module_worker_hint,
        ),
        src: script.src.clone(),
        resolved_src: script.resolved_src.clone(),
        type_hint: script.type_hint.clone(),
        execution_kind: if script.src.is_some() {
            "external".to_string()
        } else {
            "inline".to_string()
        },
        has_text: script.text.is_some(),
        text_length: text.chars().count(),
        messaging_target_count: messaging_targets.len(),
        messaging_targets,
        creates_worker,
        creates_shared_worker,
        registers_service_worker,
        uses_post_message,
        listens_message_events,
        uses_message_channel,
        uses_broadcast_channel,
        uses_import_scripts,
        module_worker_hint,
        messaging_blocked: !messaging_block_reasons.is_empty(),
        messaging_block_reasons,
    })
}

fn expected_script_worker_messaging_targets(
    creates_worker: bool,
    creates_shared_worker: bool,
    registers_service_worker: bool,
    uses_post_message: bool,
    listens_message_events: bool,
    uses_message_channel: bool,
    uses_broadcast_channel: bool,
    uses_import_scripts: bool,
    module_worker_hint: bool,
) -> Vec<String> {
    let mut targets = Vec::new();
    if creates_worker {
        targets.push("Worker".to_string());
    }
    if creates_shared_worker {
        targets.push("SharedWorker".to_string());
    }
    if registers_service_worker {
        targets.push("serviceWorker".to_string());
    }
    if uses_post_message {
        targets.push("postMessage".to_string());
    }
    if listens_message_events {
        targets.push("message-event".to_string());
    }
    if uses_message_channel {
        targets.push("MessageChannel".to_string());
    }
    if uses_broadcast_channel {
        targets.push("BroadcastChannel".to_string());
    }
    if uses_import_scripts {
        targets.push("importScripts".to_string());
    }
    if module_worker_hint {
        targets.push("module-worker".to_string());
    }
    targets
}

fn expected_script_worker_messaging_kind(
    creates_worker: bool,
    creates_shared_worker: bool,
    registers_service_worker: bool,
    uses_post_message: bool,
    listens_message_events: bool,
    uses_message_channel: bool,
    uses_broadcast_channel: bool,
    uses_import_scripts: bool,
    module_worker_hint: bool,
) -> String {
    if registers_service_worker {
        "service-worker-registration".to_string()
    } else if creates_shared_worker {
        "shared-worker".to_string()
    } else if creates_worker && module_worker_hint {
        "module-worker".to_string()
    } else if creates_worker || uses_import_scripts {
        "dedicated-worker".to_string()
    } else if uses_message_channel || uses_broadcast_channel {
        "channel-messaging".to_string()
    } else if uses_post_message || listens_message_events {
        "post-message".to_string()
    } else {
        "worker-messaging-metadata".to_string()
    }
}

fn expected_script_worker_messaging_block_reasons(script: &BrowserScript) -> Vec<String> {
    let mut reasons = Vec::new();
    if script.script_kind == "data" {
        reasons.push("non-executable-script-type".to_string());
    }
    if script.nomodule {
        reasons.push("nomodule-fallback".to_string());
    }
    reasons
}

fn expected_script_module_graph_descriptors(
    scripts: &[BrowserScript],
    resources: &[BrowserResource],
) -> Vec<BrowserScriptModuleGraphDescriptor> {
    let modulepreloads: Vec<_> = resources
        .iter()
        .filter(|resource| resource.kind == "modulepreload")
        .collect();
    scripts
        .iter()
        .filter_map(|script| expected_script_module_graph_descriptor(script, &modulepreloads))
        .collect()
}

fn expected_script_module_graph_descriptor(
    script: &BrowserScript,
    modulepreloads: &[&BrowserResource],
) -> Option<BrowserScriptModuleGraphDescriptor> {
    let text = script.text.as_deref().unwrap_or_default();
    let normalized_text = text.to_ascii_lowercase();
    let external_module_entry = script.script_kind == "module" && script.src.is_some();
    let inline_module_entry = script.script_kind == "module" && script.src.is_none();
    let declares_import_map = script.script_kind == "importmap";
    let uses_static_imports = expected_script_uses_static_module_imports(&normalized_text);
    let uses_dynamic_imports = normalized_text.contains("import(");
    let has_modulepreload = !modulepreloads.is_empty();
    let module_targets = expected_script_module_graph_targets(
        external_module_entry,
        inline_module_entry,
        declares_import_map,
        uses_static_imports,
        uses_dynamic_imports,
        has_modulepreload,
    );
    if module_targets.is_empty() {
        return None;
    }

    let modulepreload_urls = modulepreloads
        .iter()
        .map(|resource| resource.url.clone())
        .collect();
    let resolved_modulepreload_urls = modulepreloads
        .iter()
        .filter_map(|resource| resource.resolved_url.clone())
        .collect();
    let module_graph_block_reasons = expected_script_module_graph_block_reasons(script);
    Some(BrowserScriptModuleGraphDescriptor {
        script_kind: script.script_kind.clone(),
        module_graph_kind: expected_script_module_graph_kind(
            external_module_entry,
            inline_module_entry,
            declares_import_map,
            uses_static_imports,
            uses_dynamic_imports,
        ),
        src: script.src.clone(),
        resolved_src: script.resolved_src.clone(),
        type_hint: script.type_hint.clone(),
        execution_kind: if script.src.is_some() {
            "external".to_string()
        } else {
            "inline".to_string()
        },
        has_text: script.text.is_some(),
        text_length: text.chars().count(),
        module_target_count: module_targets.len(),
        module_targets,
        external_module_entry,
        inline_module_entry,
        declares_import_map,
        uses_static_imports,
        uses_dynamic_imports,
        has_modulepreload,
        modulepreload_urls,
        resolved_modulepreload_urls,
        module_graph_blocked: !module_graph_block_reasons.is_empty(),
        module_graph_block_reasons,
    })
}

fn expected_script_uses_static_module_imports(normalized_text: &str) -> bool {
    normalized_text.contains("import ")
        || normalized_text.contains("import{")
        || normalized_text.contains("export ")
        || normalized_text.contains(" from '")
        || normalized_text.contains(" from \"")
}

fn expected_script_module_graph_targets(
    external_module_entry: bool,
    inline_module_entry: bool,
    declares_import_map: bool,
    uses_static_imports: bool,
    uses_dynamic_imports: bool,
    has_modulepreload: bool,
) -> Vec<String> {
    let mut targets = Vec::new();
    if external_module_entry {
        targets.push("external-module-entry".to_string());
    }
    if inline_module_entry {
        targets.push("inline-module-entry".to_string());
    }
    if declares_import_map {
        targets.push("importmap".to_string());
    }
    if uses_static_imports {
        targets.push("static-import".to_string());
    }
    if uses_dynamic_imports {
        targets.push("dynamic-import".to_string());
    }
    if has_modulepreload {
        targets.push("modulepreload".to_string());
    }
    targets
}

fn expected_script_module_graph_kind(
    external_module_entry: bool,
    inline_module_entry: bool,
    declares_import_map: bool,
    uses_static_imports: bool,
    uses_dynamic_imports: bool,
) -> String {
    if declares_import_map {
        "import-map".to_string()
    } else if uses_static_imports && uses_dynamic_imports {
        "mixed-module-imports".to_string()
    } else if uses_static_imports {
        "static-module-graph".to_string()
    } else if uses_dynamic_imports {
        "dynamic-module-import".to_string()
    } else if external_module_entry || inline_module_entry {
        "module-entry".to_string()
    } else {
        "module-graph-metadata".to_string()
    }
}

fn expected_script_module_graph_block_reasons(script: &BrowserScript) -> Vec<String> {
    let mut reasons = Vec::new();
    if script.script_kind == "data" {
        reasons.push("non-executable-script-type".to_string());
    }
    if script.nomodule {
        reasons.push("nomodule-fallback".to_string());
    }
    reasons
}

fn expected_stylesheet_planning_descriptors(
    stylesheets: &[BrowserStylesheet],
) -> Vec<BrowserStylesheetPlanningDescriptor> {
    stylesheets
        .iter()
        .map(expected_stylesheet_planning_descriptor)
        .collect()
}

fn expected_stylesheet_planning_descriptor(
    stylesheet: &BrowserStylesheet,
) -> BrowserStylesheetPlanningDescriptor {
    let text_length = stylesheet
        .text
        .as_ref()
        .map(|text| text.chars().count())
        .unwrap_or_default();
    BrowserStylesheetPlanningDescriptor {
        source: stylesheet.source.clone(),
        stylesheet_kind: if stylesheet.href.is_some() {
            "external".to_string()
        } else {
            "inline".to_string()
        },
        href: stylesheet.href.clone(),
        resolved_href: stylesheet.resolved_href.clone(),
        rel: stylesheet.rel.clone(),
        rel_tokens: stylesheet.rel_tokens.clone(),
        rel_token_count: stylesheet.rel_tokens.len(),
        type_hint: stylesheet.type_hint.clone(),
        media: stylesheet.media.clone(),
        title: stylesheet.title.clone(),
        disabled: stylesheet.disabled,
        alternate: stylesheet.alternate,
        applies_by_default: !stylesheet.disabled && !stylesheet.alternate,
        integrity: stylesheet.integrity.clone(),
        crossorigin: stylesheet.crossorigin.clone(),
        nonce: stylesheet.nonce.clone(),
        referrerpolicy: stylesheet.referrerpolicy.clone(),
        fetchpriority: stylesheet.fetchpriority.clone(),
        blocking: stylesheet.blocking.clone(),
        blocking_tokens: stylesheet.blocking_tokens.clone(),
        blocking_token_count: stylesheet.blocking_tokens.len(),
        render_blocking: stylesheet
            .blocking_tokens
            .iter()
            .any(|token| token.eq_ignore_ascii_case("render")),
        has_text: stylesheet.text.is_some(),
        text_length,
    }
}

impl ExpectedResourceEndpointDescriptor {
    fn into_browser_resource_endpoint_descriptor(self) -> BrowserResourceEndpointDescriptor {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserResourceEndpointDescriptor {
            endpoint_kind: self.endpoint_kind,
            element: self.element,
            resource_kind: self.resource_kind,
            url: self.url,
            resolved_url: self.resolved_url,
            rel: self.rel,
            rel_tokens,
            as_hint: self.as_hint,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            sizes: self.sizes,
            hreflang: self.hreflang,
            color: self.color,
            width: self.width,
            height: self.height,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            csp: self.csp,
            blocking: self.blocking,
            blocking_tokens,
            browsing_context_name: self.browsing_context_name,
            loading: self.loading,
            sandbox: self.sandbox,
            allow: self.allow,
            allowfullscreen: self.allowfullscreen,
            srcdoc: self.srcdoc,
            credentialless: self.credentialless,
            imagesrcset: self.imagesrcset,
            resolved_imagesrcset: self.resolved_imagesrcset,
            imagesizes: self.imagesizes,
            track_kind: self.track_kind,
            srclang: self.srclang,
            track_label: self.track_label,
            default_track: self.default_track,
            async_script: self.async_script,
            defer_script: self.defer_script,
        }
    }
}

impl ExpectedScript {
    fn into_browser_script(self) -> BrowserScript {
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserScript {
            script_kind: self.script_kind,
            src: self.src,
            resolved_src: self.resolved_src,
            type_hint: self.type_hint,
            async_script: self.async_script,
            defer_script: self.defer_script,
            nomodule: self.nomodule,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            text: self.text,
        }
    }
}

impl ExpectedScriptExecutionDescriptor {
    fn into_browser_script_execution_descriptor(self) -> BrowserScriptExecutionDescriptor {
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserScriptExecutionDescriptor {
            script_kind: self.script_kind,
            execution_kind: self.execution_kind,
            src: self.src,
            resolved_src: self.resolved_src,
            type_hint: self.type_hint,
            async_script: self.async_script,
            defer_script: self.defer_script,
            nomodule: self.nomodule,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            blocking_token_count: self.blocking_token_count,
            render_blocking: self.render_blocking,
            has_text: self.has_text,
            text_length: self.text_length,
        }
    }
}

impl ExpectedScriptStorageAccessDescriptor {
    fn into_browser_script_storage_access_descriptor(self) -> BrowserScriptStorageAccessDescriptor {
        BrowserScriptStorageAccessDescriptor {
            script_kind: self.script_kind,
            access_kind: self.access_kind,
            src: self.src,
            resolved_src: self.resolved_src,
            type_hint: self.type_hint,
            execution_kind: self.execution_kind,
            has_text: self.has_text,
            text_length: self.text_length,
            storage_targets: self.storage_targets,
            storage_target_count: self.storage_target_count,
            uses_local_storage: self.uses_local_storage,
            uses_session_storage: self.uses_session_storage,
            uses_cookies: self.uses_cookies,
            uses_indexed_db: self.uses_indexed_db,
            uses_cache_storage: self.uses_cache_storage,
            uses_service_worker: self.uses_service_worker,
            uses_storage_manager: self.uses_storage_manager,
            listens_storage_events: self.listens_storage_events,
            storage_blocked: self.storage_blocked,
            storage_block_reasons: self.storage_block_reasons,
        }
    }
}

impl ExpectedScriptWorkerMessagingDescriptor {
    fn into_browser_script_worker_messaging_descriptor(
        self,
    ) -> BrowserScriptWorkerMessagingDescriptor {
        BrowserScriptWorkerMessagingDescriptor {
            script_kind: self.script_kind,
            messaging_kind: self.messaging_kind,
            src: self.src,
            resolved_src: self.resolved_src,
            type_hint: self.type_hint,
            execution_kind: self.execution_kind,
            has_text: self.has_text,
            text_length: self.text_length,
            messaging_targets: self.messaging_targets,
            messaging_target_count: self.messaging_target_count,
            creates_worker: self.creates_worker,
            creates_shared_worker: self.creates_shared_worker,
            registers_service_worker: self.registers_service_worker,
            uses_post_message: self.uses_post_message,
            listens_message_events: self.listens_message_events,
            uses_message_channel: self.uses_message_channel,
            uses_broadcast_channel: self.uses_broadcast_channel,
            uses_import_scripts: self.uses_import_scripts,
            module_worker_hint: self.module_worker_hint,
            messaging_blocked: self.messaging_blocked,
            messaging_block_reasons: self.messaging_block_reasons,
        }
    }
}

impl ExpectedScriptModuleGraphDescriptor {
    fn into_browser_script_module_graph_descriptor(self) -> BrowserScriptModuleGraphDescriptor {
        BrowserScriptModuleGraphDescriptor {
            script_kind: self.script_kind,
            module_graph_kind: self.module_graph_kind,
            src: self.src,
            resolved_src: self.resolved_src,
            type_hint: self.type_hint,
            execution_kind: self.execution_kind,
            has_text: self.has_text,
            text_length: self.text_length,
            module_targets: self.module_targets,
            module_target_count: self.module_target_count,
            external_module_entry: self.external_module_entry,
            inline_module_entry: self.inline_module_entry,
            declares_import_map: self.declares_import_map,
            uses_static_imports: self.uses_static_imports,
            uses_dynamic_imports: self.uses_dynamic_imports,
            has_modulepreload: self.has_modulepreload,
            modulepreload_urls: self.modulepreload_urls,
            resolved_modulepreload_urls: self.resolved_modulepreload_urls,
            module_graph_blocked: self.module_graph_blocked,
            module_graph_block_reasons: self.module_graph_block_reasons,
        }
    }
}

impl ExpectedStylesheet {
    fn into_browser_stylesheet(self) -> BrowserStylesheet {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserStylesheet {
            source: self.source,
            href: self.href,
            resolved_href: self.resolved_href,
            rel: self.rel,
            rel_tokens,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            disabled: self.disabled,
            alternate: self.alternate,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            text: self.text,
        }
    }
}

impl ExpectedStylesheetPlanningDescriptor {
    fn into_browser_stylesheet_planning_descriptor(self) -> BrowserStylesheetPlanningDescriptor {
        let rel_tokens = expected_tokens_from_raw(self.rel_tokens, self.rel.as_deref());
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserStylesheetPlanningDescriptor {
            source: self.source,
            stylesheet_kind: self.stylesheet_kind,
            href: self.href,
            resolved_href: self.resolved_href,
            rel: self.rel,
            rel_tokens,
            rel_token_count: self.rel_token_count,
            type_hint: self.type_hint,
            media: self.media,
            title: self.title,
            disabled: self.disabled,
            alternate: self.alternate,
            applies_by_default: self.applies_by_default,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            blocking_token_count: self.blocking_token_count,
            render_blocking: self.render_blocking,
            has_text: self.has_text,
            text_length: self.text_length,
        }
    }
}

impl ExpectedLoadingHintDescriptor {
    fn into_browser_loading_hint_descriptor(self) -> BrowserLoadingHintDescriptor {
        let blocking_tokens =
            expected_tokens_from_raw(self.blocking_tokens, self.blocking.as_deref());
        BrowserLoadingHintDescriptor {
            element: self.element,
            id: self.id,
            url: self.url,
            resolved_url: self.resolved_url,
            loading: self.loading,
            decoding: self.decoding,
            fetchpriority: self.fetchpriority,
            blocking: self.blocking,
            blocking_tokens,
            preload: self.preload,
            as_hint: self.as_hint,
            media: self.media,
        }
    }
}

impl ExpectedFetchPolicyDescriptor {
    fn into_browser_fetch_policy_descriptor(self) -> BrowserFetchPolicyDescriptor {
        BrowserFetchPolicyDescriptor {
            element: self.element,
            id: self.id,
            url: self.url,
            resolved_url: self.resolved_url,
            integrity: self.integrity,
            crossorigin: self.crossorigin,
            nonce: self.nonce,
            referrerpolicy: self.referrerpolicy,
            csp: self.csp,
            sandbox: self.sandbox,
            allow: self.allow,
            allowfullscreen: self.allowfullscreen,
            credentialless: self.credentialless,
        }
    }
}

impl ExpectedDocumentPolicyDescriptor {
    fn into_browser_document_policy_descriptor(self) -> BrowserDocumentPolicyDescriptor {
        BrowserDocumentPolicyDescriptor {
            charset: self.charset,
            viewport: self.viewport,
            viewport_directives: self
                .viewport_directives
                .into_iter()
                .map(ExpectedMetadataDirective::into_browser_metadata_directive)
                .collect(),
            description: self.description,
            application_name: self.application_name,
            referrer_policy: self.referrer_policy,
            robots: self.robots,
            robots_directives: self.robots_directives,
            color_scheme: self.color_scheme,
            content_security_policy: self.content_security_policy,
            permissions_policy: self.permissions_policy,
            permissions_policy_features: self.permissions_policy_features,
            permissions_policy_feature_count: self.permissions_policy_feature_count,
            origin_trials: self.origin_trials,
            accept_ch: self.accept_ch,
            accept_ch_tokens: self.accept_ch_tokens,
            dns_prefetch_control: self.dns_prefetch_control,
            theme_colors: self
                .theme_colors
                .into_iter()
                .map(ExpectedThemeColor::into_browser_theme_color)
                .collect(),
            refresh: self.refresh.map(ExpectedRefresh::into_browser_refresh),
            canonical_url: self.canonical_url,
            resolved_canonical_url: self.resolved_canonical_url,
            manifest_url: self.manifest_url,
            resolved_manifest_url: self.resolved_manifest_url,
        }
    }
}

impl ExpectedNavigationTargetDescriptor {
    fn into_browser_navigation_target_descriptor(self) -> BrowserNavigationTargetDescriptor {
        BrowserNavigationTargetDescriptor {
            element: self.element,
            id: self.id,
            href: self.href,
            resolved_href: self.resolved_href,
            text: self.text,
            target: self.target,
            effective_target: self.effective_target,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            rel_external: self.rel_external,
            rel_nofollow: self.rel_nofollow,
            rel_noopener: self.rel_noopener,
            rel_noreferrer: self.rel_noreferrer,
            download: self.download,
            ping: self.ping,
            resolved_ping: self.resolved_ping,
            attributionsrc: self.attributionsrc,
            resolved_attributionsrc: self.resolved_attributionsrc,
            hreflang: self.hreflang,
            type_hint: self.type_hint,
            referrerpolicy: self.referrerpolicy,
            area_shape: self.area_shape,
            area_coords: self.area_coords,
        }
    }
}

impl ExpectedFormPolicyDescriptor {
    fn into_browser_form_policy_descriptor(self) -> BrowserFormPolicyDescriptor {
        BrowserFormPolicyDescriptor {
            id: self.id,
            name: self.name,
            action: self.action,
            resolved_action: self.resolved_action,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            accept_charset: self.accept_charset,
            accept_charset_tokens: self.accept_charset_tokens,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            rel_external: self.rel_external,
            rel_nofollow: self.rel_nofollow,
            rel_noopener: self.rel_noopener,
            rel_noreferrer: self.rel_noreferrer,
            novalidate: self.novalidate,
            submitters: self
                .submitters
                .into_iter()
                .map(
                    ExpectedFormPolicySubmitterDescriptor::into_browser_form_policy_submitter_descriptor,
                )
                .collect(),
        }
    }
}

impl ExpectedFormPolicySubmitterDescriptor {
    fn into_browser_form_policy_submitter_descriptor(self) -> BrowserFormPolicySubmitterDescriptor {
        BrowserFormPolicySubmitterDescriptor {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            accessible_name: self.accessible_name,
            action: self.action,
            resolved_action: self.resolved_action,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            novalidate: self.novalidate,
            value: self.value,
        }
    }
}

impl ExpectedFormControlDescriptor {
    fn into_browser_form_control_descriptor(self) -> BrowserFormControlDescriptor {
        BrowserFormControlDescriptor {
            form_id: self.form_id,
            form_name: self.form_name,
            element: self.element,
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            control_kind: self.control_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            labels: self.labels,
            label_count: self.label_count,
            value: self.value,
            submission_values: self.submission_values,
            submission_value_count: self.submission_value_count,
            placeholder: self.placeholder,
            autocomplete_tokens: self.autocomplete_tokens,
            datalist_options: self.datalist_options,
            option_count: self.option_count,
            selected_options: self.selected_options,
            checked: self.checked,
            multiple: self.multiple,
            autofocus: self.autofocus,
            disabled: self.disabled,
            required: self.required,
            readonly: self.readonly,
            successful: self.successful,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
            fieldset_ids: self.fieldset_ids,
            fieldset_legends: self.fieldset_legends,
            control_blocked: self.control_blocked,
            control_block_reasons: self.control_block_reasons,
        }
    }
}

fn expected_form_control_descriptors(forms: &[BrowserForm]) -> Vec<BrowserFormControlDescriptor> {
    forms
        .iter()
        .flat_map(|form| {
            form.controls
                .iter()
                .map(|control| expected_form_control_descriptor(form, control))
        })
        .collect()
}

fn expected_form_control_descriptor(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> BrowserFormControlDescriptor {
    let fieldset_ids = expected_form_association_fieldset_ids(form, control);
    let fieldset_legends = expected_form_association_fieldset_legends(form, control);
    let control_block_reasons = expected_form_control_block_reasons(control);

    BrowserFormControlDescriptor {
        form_id: form.id.clone(),
        form_name: form.name.clone(),
        element: expected_form_association_element(control),
        id: control.id.clone(),
        control_type: control.control_type.clone(),
        name: control.name.clone(),
        form_owner: control.form_owner.clone(),
        control_kind: expected_form_control_kind(control, &control_block_reasons),
        text: control.text.clone(),
        accessible_name: control
            .accessible_name
            .clone()
            .or_else(|| control.alt.clone()),
        accessible_description: control.accessible_description.clone(),
        label_count: control.labels.len(),
        labels: control.labels.clone(),
        value: control.value.clone(),
        submission_value_count: control.submission_values.len(),
        submission_values: control.submission_values.clone(),
        placeholder: control.placeholder.clone(),
        autocomplete_tokens: control.autocomplete_tokens.clone(),
        datalist_options: control.datalist_options.clone(),
        option_count: control.option_items.len(),
        selected_options: control.selected_options.clone(),
        checked: control.checked,
        multiple: control.multiple,
        autofocus: control.autofocus,
        disabled: control.disabled,
        required: control.required,
        readonly: control.readonly,
        successful: control.successful,
        will_validate: control.will_validate,
        validation_attributes: control.validation_attributes.clone(),
        validation_barred_reason: control.validation_barred_reason.clone(),
        fieldset_ids,
        fieldset_legends,
        control_blocked: !control_block_reasons.is_empty(),
        control_block_reasons,
    }
}

fn expected_form_control_kind(
    control: &BrowserFormControl,
    control_block_reasons: &[String],
) -> String {
    if !control_block_reasons.is_empty() {
        "blocked-control".to_string()
    } else if expected_form_submitter(control) {
        "submitter-control".to_string()
    } else if control.control_type == "select" {
        "selection-control".to_string()
    } else if matches!(control.control_type.as_str(), "checkbox" | "radio") {
        "choice-control".to_string()
    } else if control.control_type == "file" {
        "file-control".to_string()
    } else if control.control_type == "hidden" {
        "hidden-control".to_string()
    } else if control.control_type == "output" {
        "output-control".to_string()
    } else if control.successful {
        "successful-control".to_string()
    } else {
        "form-control".to_string()
    }
}

fn expected_form_control_block_reasons(control: &BrowserFormControl) -> Vec<String> {
    let mut reasons = Vec::new();
    if control.disabled {
        reasons.push("disabled".to_string());
    }
    if control.name.is_none() && expected_form_control_needs_name(control) {
        reasons.push("missing-name".to_string());
    }
    if matches!(control.control_type.as_str(), "checkbox" | "radio") && !control.checked {
        reasons.push("unchecked-choice".to_string());
    }
    if control.readonly {
        reasons.push("readonly".to_string());
    }
    if let Some(reason) = &control.validation_barred_reason {
        let reason = format!("validation-barred:{reason}");
        if !reasons.iter().any(|existing| existing == &reason) {
            reasons.push(reason);
        }
    }
    reasons
}

fn expected_form_control_needs_name(control: &BrowserFormControl) -> bool {
    !matches!(control.control_type.as_str(), "button" | "output" | "reset")
}

impl ExpectedFormAssociationDescriptor {
    fn into_browser_form_association_descriptor(self) -> BrowserFormAssociationDescriptor {
        BrowserFormAssociationDescriptor {
            form_id: self.form_id,
            form_name: self.form_name,
            element: self.element,
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            association_kind: self.association_kind,
            explicit_form_owner: self.explicit_form_owner,
            labels: self.labels,
            label_count: self.label_count,
            fieldset_ids: self.fieldset_ids,
            fieldset_legends: self.fieldset_legends,
            datalist_id: self.datalist_id,
            datalist_option_count: self.datalist_option_count,
            output_for_tokens: self.output_for_tokens,
            output_target_ids: self.output_target_ids,
            output_target_names: self.output_target_names,
            output_target_types: self.output_target_types,
            referenced_by_output_ids: self.referenced_by_output_ids,
            successful: self.successful,
            will_validate: self.will_validate,
            disabled: self.disabled,
        }
    }
}

fn expected_form_association_descriptors(
    forms: &[BrowserForm],
) -> Vec<BrowserFormAssociationDescriptor> {
    forms
        .iter()
        .flat_map(|form| {
            form.controls
                .iter()
                .map(|control| expected_form_association_descriptor(form, control))
        })
        .collect()
}

fn expected_form_association_descriptor(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> BrowserFormAssociationDescriptor {
    let fieldset_ids = expected_form_association_fieldset_ids(form, control);
    let fieldset_legends = expected_form_association_fieldset_legends(form, control);
    let datalist_option_count = expected_form_association_datalist_option_count(form, control);
    let output_targets = expected_output_for_controls(&form.controls, control);
    let output_target_ids = output_targets
        .iter()
        .filter_map(|target| target.id.clone())
        .collect();
    let output_target_names = output_targets
        .iter()
        .filter_map(|target| target.name.clone())
        .collect();
    let output_target_types = output_targets
        .iter()
        .map(|target| target.control_type.clone())
        .collect();
    let referenced_by_output_ids =
        expected_form_association_referenced_by_output_ids(form, control);

    BrowserFormAssociationDescriptor {
        form_id: form.id.clone(),
        form_name: form.name.clone(),
        element: expected_form_association_element(control),
        id: control.id.clone(),
        control_type: control.control_type.clone(),
        name: control.name.clone(),
        form_owner: control.form_owner.clone(),
        association_kind: expected_form_association_kind(
            form,
            control,
            &fieldset_ids,
            datalist_option_count,
            &referenced_by_output_ids,
        ),
        explicit_form_owner: expected_form_association_explicit_owner(form, control),
        label_count: control.labels.len(),
        labels: control.labels.clone(),
        fieldset_ids,
        fieldset_legends,
        datalist_id: control.list.clone(),
        datalist_option_count,
        output_for_tokens: control.output_for.clone(),
        output_target_ids,
        output_target_names,
        output_target_types,
        referenced_by_output_ids,
        successful: control.successful,
        will_validate: control.will_validate,
        disabled: control.disabled,
    }
}

fn expected_form_association_kind(
    form: &BrowserForm,
    control: &BrowserFormControl,
    fieldset_ids: &[String],
    datalist_option_count: usize,
    referenced_by_output_ids: &[String],
) -> String {
    if expected_form_association_explicit_owner(form, control) {
        "explicit-form-owner".to_string()
    } else if !control.output_for.is_empty() {
        "output-calculation".to_string()
    } else if !referenced_by_output_ids.is_empty() {
        "output-source".to_string()
    } else if datalist_option_count > 0 {
        "datalist-backed-control".to_string()
    } else if !control.labels.is_empty() {
        "labelled-control".to_string()
    } else if !fieldset_ids.is_empty() {
        "fieldset-member".to_string()
    } else {
        "form-associated-control".to_string()
    }
}

fn expected_form_association_explicit_owner(
    _form: &BrowserForm,
    control: &BrowserFormControl,
) -> bool {
    control.form_owner.is_some()
}

fn expected_form_association_fieldset_ids(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> Vec<String> {
    form.fieldsets
        .iter()
        .filter(|fieldset| expected_fieldset_contains_control(fieldset, control))
        .filter_map(|fieldset| fieldset.id.clone())
        .collect()
}

fn expected_form_association_fieldset_legends(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> Vec<String> {
    form.fieldsets
        .iter()
        .filter(|fieldset| expected_fieldset_contains_control(fieldset, control))
        .filter_map(|fieldset| fieldset.legend.clone())
        .collect()
}

fn expected_fieldset_contains_control(
    fieldset: &BrowserFormFieldset,
    control: &BrowserFormControl,
) -> bool {
    control.id.as_deref().is_some_and(|id| {
        fieldset
            .control_ids
            .iter()
            .any(|control_id| control_id == id)
    }) || control.name.as_deref().is_some_and(|name| {
        fieldset
            .control_names
            .iter()
            .any(|control_name| control_name == name)
    })
}

fn expected_form_association_datalist_option_count(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> usize {
    control
        .list
        .as_deref()
        .and_then(|list| {
            form.datalists
                .iter()
                .find(|datalist| datalist.id.as_deref() == Some(list))
        })
        .map(|datalist| datalist.options.len())
        .unwrap_or_default()
}

fn expected_form_association_referenced_by_output_ids(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> Vec<String> {
    form.outputs
        .iter()
        .filter(|output| {
            control
                .id
                .as_deref()
                .is_some_and(|id| output.for_control_ids.iter().any(|target| target == id))
                || control.name.as_deref().is_some_and(|name| {
                    output.for_control_names.iter().any(|target| target == name)
                })
        })
        .filter_map(|output| output.id.clone())
        .collect()
}

fn expected_output_for_controls<'a>(
    controls: &'a [BrowserFormControl],
    output: &BrowserFormControl,
) -> Vec<&'a BrowserFormControl> {
    output
        .output_for
        .iter()
        .filter_map(|token| {
            controls
                .iter()
                .find(|control| control.id.as_deref() == Some(token.as_str()))
        })
        .collect()
}

fn expected_form_association_element(control: &BrowserFormControl) -> String {
    match control.control_type.as_str() {
        "button" | "checkbox" | "color" | "date" | "datetime-local" | "email" | "file"
        | "hidden" | "image" | "month" | "number" | "password" | "radio" | "range" | "reset"
        | "search" | "submit" | "tel" | "text" | "time" | "url" | "week" => "input".to_string(),
        other => other.to_string(),
    }
}

impl ExpectedFormAutofillDescriptor {
    fn into_browser_form_autofill_descriptor(self) -> BrowserFormAutofillDescriptor {
        BrowserFormAutofillDescriptor {
            form_id: self.form_id,
            form_name: self.form_name,
            form_autocomplete: self.form_autocomplete,
            form_autocomplete_tokens: self.form_autocomplete_tokens,
            form_autocomplete_enabled: self.form_autocomplete_enabled,
            element: self.element,
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            autofill_kind: self.autofill_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            value: self.value,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            autocomplete_token_count: self.autocomplete_token_count,
            section_token: self.section_token,
            address_type_token: self.address_type_token,
            contact_type_token: self.contact_type_token,
            field_token: self.field_token,
            webauthn: self.webauthn,
            autofill_enabled: self.autofill_enabled,
            disabled: self.disabled,
            readonly: self.readonly,
            hidden: self.hidden,
            required: self.required,
            autofill_blocked: self.autofill_blocked,
            autofill_block_reasons: self.autofill_block_reasons,
        }
    }
}

fn expected_form_autofill_descriptors(forms: &[BrowserForm]) -> Vec<BrowserFormAutofillDescriptor> {
    forms
        .iter()
        .flat_map(|form| {
            form.controls
                .iter()
                .filter_map(|control| expected_form_autofill_descriptor(form, control))
        })
        .collect()
}

fn expected_form_autofill_descriptor(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> Option<BrowserFormAutofillDescriptor> {
    if !expected_form_autofill_candidate(form, control) {
        return None;
    }

    let section_token = expected_autofill_section_token(&control.autocomplete_tokens);
    let address_type_token = expected_autofill_address_type_token(&control.autocomplete_tokens);
    let contact_type_token = expected_autofill_contact_type_token(&control.autocomplete_tokens);
    let field_token = expected_autofill_field_token(&control.autocomplete_tokens);
    let webauthn = expected_autofill_has_webauthn_token(&control.autocomplete_tokens);
    let autofill_block_reasons = expected_form_autofill_block_reasons(form, control);
    Some(BrowserFormAutofillDescriptor {
        form_id: form.id.clone(),
        form_name: form.name.clone(),
        form_autocomplete: form.autocomplete.clone(),
        form_autocomplete_tokens: form.autocomplete_tokens.clone(),
        form_autocomplete_enabled: !expected_autofill_tokens_are_off(&form.autocomplete_tokens),
        element: expected_form_autofill_element(control),
        id: control.id.clone(),
        control_type: control.control_type.clone(),
        name: control.name.clone(),
        form_owner: control.form_owner.clone(),
        autofill_kind: expected_form_autofill_kind(
            form,
            control,
            field_token.as_deref(),
            webauthn,
            &autofill_block_reasons,
        ),
        text: control.text.clone(),
        accessible_name: control
            .accessible_name
            .clone()
            .or_else(|| control.alt.clone()),
        value: control.value.clone(),
        autocomplete: control.autocomplete.clone(),
        autocomplete_token_count: control.autocomplete_tokens.len(),
        autocomplete_tokens: control.autocomplete_tokens.clone(),
        section_token,
        address_type_token,
        contact_type_token,
        field_token,
        webauthn,
        autofill_enabled: autofill_block_reasons.is_empty(),
        disabled: control.disabled,
        readonly: control.readonly,
        hidden: control.control_type == "hidden",
        required: control.required,
        autofill_blocked: !autofill_block_reasons.is_empty(),
        autofill_block_reasons,
    })
}

fn expected_form_autofill_candidate(form: &BrowserForm, control: &BrowserFormControl) -> bool {
    !control.autocomplete_tokens.is_empty()
        || !form.autocomplete_tokens.is_empty()
        || expected_form_autofill_control_type(control)
        || expected_form_common_autofill_name(control)
}

fn expected_form_autofill_control_type(control: &BrowserFormControl) -> bool {
    matches!(
        control.control_type.as_str(),
        "color"
            | "date"
            | "datetime-local"
            | "email"
            | "hidden"
            | "month"
            | "number"
            | "password"
            | "search"
            | "select"
            | "tel"
            | "text"
            | "textarea"
            | "time"
            | "url"
            | "week"
    )
}

fn expected_form_common_autofill_name(control: &BrowserFormControl) -> bool {
    let Some(name) = control.name.as_deref() else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    [
        "address",
        "address1",
        "address2",
        "city",
        "country",
        "email",
        "family-name",
        "given-name",
        "name",
        "organization",
        "postal-code",
        "state",
        "street-address",
        "tel",
        "username",
        "zip",
    ]
    .iter()
    .any(|candidate| name == *candidate || name.contains(candidate))
}

fn expected_form_autofill_kind(
    form: &BrowserForm,
    control: &BrowserFormControl,
    field_token: Option<&str>,
    webauthn: bool,
    autofill_block_reasons: &[String],
) -> String {
    if !autofill_block_reasons.is_empty() {
        "blocked-autofill".to_string()
    } else if webauthn {
        "webauthn-field".to_string()
    } else if field_token.is_some() {
        "autocomplete-field".to_string()
    } else if expected_autofill_tokens_are_off(&control.autocomplete_tokens) {
        "autocomplete-off".to_string()
    } else if !form.autocomplete_tokens.is_empty() {
        "form-autocomplete".to_string()
    } else if expected_form_text_autofill_control(control) {
        "text-entry-autofill".to_string()
    } else if control.control_type == "select" {
        "choice-autofill".to_string()
    } else if control.control_type == "hidden" {
        "hidden-autofill-metadata".to_string()
    } else {
        "autofill-metadata".to_string()
    }
}

fn expected_form_text_autofill_control(control: &BrowserFormControl) -> bool {
    matches!(
        control.control_type.as_str(),
        "color"
            | "date"
            | "datetime-local"
            | "email"
            | "month"
            | "number"
            | "password"
            | "search"
            | "tel"
            | "text"
            | "textarea"
            | "time"
            | "url"
            | "week"
    )
}

fn expected_form_autofill_block_reasons(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if control.disabled {
        reasons.push("disabled".to_string());
    }
    if control.readonly && expected_form_text_autofill_control(control) {
        reasons.push("readonly".to_string());
    }
    if control.control_type == "hidden" {
        reasons.push("hidden".to_string());
    }
    if expected_form_autofill_effective_off(form, control) {
        reasons.push("autocomplete-off".to_string());
    }
    reasons
}

fn expected_form_autofill_effective_off(form: &BrowserForm, control: &BrowserFormControl) -> bool {
    if !control.autocomplete_tokens.is_empty() {
        return expected_autofill_tokens_are_off(&control.autocomplete_tokens);
    }
    expected_autofill_tokens_are_off(&form.autocomplete_tokens)
}

fn expected_autofill_tokens_are_off(tokens: &[String]) -> bool {
    tokens.len() == 1 && tokens[0].eq_ignore_ascii_case("off")
}

fn expected_autofill_section_token(tokens: &[String]) -> Option<String> {
    tokens
        .iter()
        .find(|token| token.to_ascii_lowercase().starts_with("section-"))
        .cloned()
}

fn expected_autofill_address_type_token(tokens: &[String]) -> Option<String> {
    tokens
        .iter()
        .find(|token| matches!(token.to_ascii_lowercase().as_str(), "shipping" | "billing"))
        .cloned()
}

fn expected_autofill_contact_type_token(tokens: &[String]) -> Option<String> {
    tokens
        .iter()
        .find(|token| {
            matches!(
                token.to_ascii_lowercase().as_str(),
                "home" | "work" | "mobile" | "fax" | "pager"
            )
        })
        .cloned()
}

fn expected_autofill_field_token(tokens: &[String]) -> Option<String> {
    tokens
        .iter()
        .rev()
        .find(|token| expected_autofill_token_is_field(token))
        .cloned()
}

fn expected_autofill_has_webauthn_token(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|token| token.eq_ignore_ascii_case("webauthn"))
}

fn expected_autofill_token_is_field(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    if lower.starts_with("section-")
        || matches!(
            lower.as_str(),
            "on" | "off"
                | "shipping"
                | "billing"
                | "home"
                | "work"
                | "mobile"
                | "fax"
                | "pager"
                | "webauthn"
        )
    {
        return false;
    }
    true
}

fn expected_form_autofill_element(control: &BrowserFormControl) -> String {
    match control.control_type.as_str() {
        "button" | "checkbox" | "color" | "date" | "datetime-local" | "email" | "file"
        | "hidden" | "image" | "month" | "number" | "password" | "radio" | "range" | "reset"
        | "search" | "submit" | "tel" | "text" | "time" | "url" | "week" => "input".to_string(),
        other => other.to_string(),
    }
}

impl ExpectedFormSubmissionDescriptor {
    fn into_browser_form_submission_descriptor(self) -> BrowserFormSubmissionDescriptor {
        BrowserFormSubmissionDescriptor {
            form_id: self.form_id,
            form_name: self.form_name,
            form_action: self.form_action,
            resolved_form_action: self.resolved_form_action,
            form_method: self.form_method,
            form_enctype: self.form_enctype,
            form_target: self.form_target,
            effective_form_target: self.effective_form_target,
            element: self.element,
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            submission_kind: self.submission_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            value: self.value,
            submission_values: self.submission_values,
            submission_value_count: self.submission_value_count,
            successful: self.successful,
            checked: self.checked,
            disabled: self.disabled,
            submitter: self.submitter,
            submitter_action: self.submitter_action,
            resolved_submitter_action: self.resolved_submitter_action,
            submitter_method: self.submitter_method,
            submitter_enctype: self.submitter_enctype,
            submitter_target: self.submitter_target,
            effective_submitter_target: self.effective_submitter_target,
            submitter_novalidate: self.submitter_novalidate,
        }
    }
}

impl ExpectedFormResetDescriptor {
    fn into_browser_form_reset_descriptor(self) -> BrowserFormResetDescriptor {
        BrowserFormResetDescriptor {
            form_id: self.form_id,
            form_name: self.form_name,
            form_autocomplete: self.form_autocomplete,
            form_event_handlers: self.form_event_handlers,
            form_reset_handlers: self.form_reset_handlers,
            form_has_reset_handler: self.form_has_reset_handler,
            element: self.element,
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            reset_kind: self.reset_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            value: self.value,
            reset_values: self.reset_values,
            reset_value_count: self.reset_value_count,
            selected_options: self.selected_options,
            option_count: self.option_count,
            checked: self.checked,
            disabled: self.disabled,
            readonly: self.readonly,
            resettable: self.resettable,
            resetter: self.resetter,
            reset_blocked: self.reset_blocked,
            reset_block_reasons: self.reset_block_reasons,
        }
    }
}

fn expected_form_reset_descriptors(forms: &[BrowserForm]) -> Vec<BrowserFormResetDescriptor> {
    forms
        .iter()
        .flat_map(|form| {
            form.controls
                .iter()
                .filter_map(|control| expected_form_reset_descriptor(form, control))
        })
        .collect()
}

fn expected_form_reset_descriptor(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> Option<BrowserFormResetDescriptor> {
    let resettable = expected_form_resettable_control(control);
    let resetter = expected_form_resetter(control);
    if !resettable && !resetter {
        return None;
    }

    let reset_block_reasons = expected_form_reset_block_reasons(control, resettable, resetter);
    Some(BrowserFormResetDescriptor {
        form_id: form.id.clone(),
        form_name: form.name.clone(),
        form_autocomplete: form.autocomplete.clone(),
        form_event_handlers: form.event_handlers.clone(),
        form_reset_handlers: expected_event_handlers_by_kind(
            &form.event_handlers,
            expected_reset_event,
        ),
        form_has_reset_handler: !expected_event_handlers_by_kind(
            &form.event_handlers,
            expected_reset_event,
        )
        .is_empty(),
        element: expected_form_reset_element(control),
        id: control.id.clone(),
        control_type: control.control_type.clone(),
        name: control.name.clone(),
        form_owner: control.form_owner.clone(),
        reset_kind: expected_form_reset_kind(control, resettable, resetter, &reset_block_reasons),
        text: control.text.clone(),
        accessible_name: control
            .accessible_name
            .clone()
            .or_else(|| control.alt.clone()),
        value: control.value.clone(),
        reset_value_count: expected_form_reset_values(control).len(),
        reset_values: expected_form_reset_values(control),
        selected_options: control.selected_options.clone(),
        option_count: control.option_items.len(),
        checked: control.checked,
        disabled: control.disabled,
        readonly: control.readonly,
        resettable,
        resetter,
        reset_blocked: !reset_block_reasons.is_empty(),
        reset_block_reasons,
    })
}

fn expected_form_reset_kind(
    control: &BrowserFormControl,
    resettable: bool,
    resetter: bool,
    reset_block_reasons: &[String],
) -> String {
    if !reset_block_reasons.is_empty() && resetter {
        "blocked-resetter".to_string()
    } else if resetter {
        "resetter".to_string()
    } else if matches!(control.control_type.as_str(), "checkbox" | "radio") {
        "checked-reset-state".to_string()
    } else if control.control_type == "select" {
        "selection-reset-state".to_string()
    } else if control.control_type == "file" {
        "file-reset-state".to_string()
    } else if control.control_type == "output" {
        "output-reset-value".to_string()
    } else if resettable {
        "value-reset-state".to_string()
    } else {
        "reset-metadata".to_string()
    }
}

fn expected_form_resettable_control(control: &BrowserFormControl) -> bool {
    matches!(
        control.control_type.as_str(),
        "checkbox"
            | "color"
            | "date"
            | "datetime-local"
            | "email"
            | "file"
            | "month"
            | "number"
            | "password"
            | "radio"
            | "range"
            | "search"
            | "select"
            | "tel"
            | "text"
            | "textarea"
            | "time"
            | "url"
            | "week"
            | "output"
    )
}

fn expected_form_resetter(control: &BrowserFormControl) -> bool {
    control.control_type == "reset"
}

fn expected_form_reset_block_reasons(
    control: &BrowserFormControl,
    resettable: bool,
    resetter: bool,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if control.disabled {
        reasons.push("disabled".to_string());
    }
    if !resettable && !resetter {
        reasons.push("not-resettable".to_string());
    }
    reasons
}

fn expected_form_reset_values(control: &BrowserFormControl) -> Vec<String> {
    if !control.selected_options.is_empty() {
        return control.selected_options.clone();
    }
    if let Some(value) = &control.value {
        return vec![value.clone()];
    }
    if !control.text.is_empty() {
        return vec![control.text.clone()];
    }
    Vec::new()
}

fn expected_form_reset_element(control: &BrowserFormControl) -> String {
    match control.control_type.as_str() {
        "button" | "checkbox" | "color" | "date" | "datetime-local" | "email" | "file"
        | "hidden" | "image" | "month" | "number" | "password" | "radio" | "range" | "reset"
        | "search" | "submit" | "tel" | "text" | "time" | "url" | "week" => "input".to_string(),
        other => other.to_string(),
    }
}

fn expected_reset_event(handler: &str) -> bool {
    handler.eq_ignore_ascii_case("onreset")
}

fn expected_form_submission_descriptors(
    forms: &[BrowserForm],
) -> Vec<BrowserFormSubmissionDescriptor> {
    forms
        .iter()
        .flat_map(|form| {
            form.controls
                .iter()
                .filter_map(|control| expected_form_submission_descriptor(form, control))
        })
        .collect()
}

fn expected_form_submission_descriptor(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> Option<BrowserFormSubmissionDescriptor> {
    if !control.successful && !expected_form_submitter(control) {
        return None;
    }

    Some(BrowserFormSubmissionDescriptor {
        form_id: form.id.clone(),
        form_name: form.name.clone(),
        form_action: form.action.clone(),
        resolved_form_action: form.resolved_action.clone(),
        form_method: form.method.clone(),
        form_enctype: form.enctype.clone(),
        form_target: form.target.clone(),
        effective_form_target: form.effective_target.clone(),
        element: expected_form_submission_element(control),
        id: control.id.clone(),
        control_type: control.control_type.clone(),
        name: control.name.clone(),
        form_owner: control.form_owner.clone(),
        submission_kind: expected_form_submission_kind(control),
        text: control.text.clone(),
        accessible_name: control
            .accessible_name
            .clone()
            .or_else(|| control.alt.clone()),
        value: control.value.clone(),
        submission_value_count: control.submission_values.len(),
        submission_values: control.submission_values.clone(),
        successful: control.successful,
        checked: control.checked,
        disabled: control.disabled,
        submitter: expected_form_submitter(control),
        submitter_action: control.form_action.clone(),
        resolved_submitter_action: control.resolved_form_action.clone(),
        submitter_method: control.form_method.clone(),
        submitter_enctype: control.form_enctype.clone(),
        submitter_target: control.form_target.clone(),
        effective_submitter_target: control
            .form_target
            .clone()
            .or_else(|| form.target.clone())
            .or_else(|| form.effective_target.clone()),
        submitter_novalidate: form.novalidate || control.form_novalidate,
    })
}

fn expected_form_submission_kind(control: &BrowserFormControl) -> String {
    if expected_form_submitter(control) && control.successful {
        "successful-submitter".to_string()
    } else if expected_form_submitter(control) {
        "submitter".to_string()
    } else if control.successful {
        "successful-control".to_string()
    } else {
        "submission-metadata".to_string()
    }
}

fn expected_form_submitter(control: &BrowserFormControl) -> bool {
    matches!(
        control.control_type.as_str(),
        "submit" | "image" | "button" | "reset"
    )
}

fn expected_form_submission_element(control: &BrowserFormControl) -> String {
    match control.control_type.as_str() {
        "button" | "checkbox" | "color" | "date" | "datetime-local" | "email" | "file"
        | "hidden" | "image" | "month" | "number" | "password" | "radio" | "range" | "reset"
        | "search" | "submit" | "tel" | "text" | "time" | "url" | "week" => "input".to_string(),
        other => other.to_string(),
    }
}

impl ExpectedFormValidationDescriptor {
    fn into_browser_form_validation_descriptor(self) -> BrowserFormValidationDescriptor {
        BrowserFormValidationDescriptor {
            form_id: self.form_id,
            form_name: self.form_name,
            form_novalidate: self.form_novalidate,
            element: self.element,
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            validation_kind: self.validation_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            labels: self.labels,
            value: self.value,
            checked: self.checked,
            required: self.required,
            disabled: self.disabled,
            readonly: self.readonly,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_attribute_count: self.validation_attribute_count,
            validation_barred_reason: self.validation_barred_reason,
            validation_blocked: self.validation_blocked,
            validation_block_reasons: self.validation_block_reasons,
            submitter_ids: self.submitter_ids,
            submitter_novalidate_ids: self.submitter_novalidate_ids,
        }
    }
}

fn expected_form_validation_descriptors(
    forms: &[BrowserForm],
) -> Vec<BrowserFormValidationDescriptor> {
    forms
        .iter()
        .flat_map(|form| {
            form.controls
                .iter()
                .filter_map(|control| expected_form_validation_descriptor(form, control))
        })
        .collect()
}

fn expected_form_validation_descriptor(
    form: &BrowserForm,
    control: &BrowserFormControl,
) -> Option<BrowserFormValidationDescriptor> {
    if !expected_form_control_has_validation_state(control) {
        return None;
    }

    let validation_block_reasons = expected_form_validation_block_reasons(control);
    let submitter_ids = expected_form_submitter_ids(form);
    let submitter_novalidate_ids = expected_form_submitter_novalidate_ids(form);
    Some(BrowserFormValidationDescriptor {
        form_id: form.id.clone(),
        form_name: form.name.clone(),
        form_novalidate: form.novalidate,
        element: expected_form_validation_element(control),
        id: control.id.clone(),
        control_type: control.control_type.clone(),
        name: control.name.clone(),
        form_owner: control.form_owner.clone(),
        validation_kind: expected_form_validation_kind(
            control,
            form.novalidate,
            &submitter_novalidate_ids,
        ),
        text: control.text.clone(),
        accessible_name: control
            .accessible_name
            .clone()
            .or_else(|| control.alt.clone()),
        accessible_description: control.accessible_description.clone(),
        labels: control.labels.clone(),
        value: control.value.clone(),
        checked: control.checked,
        required: control.required,
        disabled: control.disabled,
        readonly: control.readonly,
        will_validate: control.will_validate,
        validation_attribute_count: control.validation_attributes.len(),
        validation_attributes: control.validation_attributes.clone(),
        validation_barred_reason: control.validation_barred_reason.clone(),
        validation_blocked: !validation_block_reasons.is_empty(),
        validation_block_reasons,
        submitter_ids,
        submitter_novalidate_ids,
    })
}

fn expected_form_control_has_validation_state(control: &BrowserFormControl) -> bool {
    control.will_validate
        || control.required
        || !control.validation_attributes.is_empty()
        || control.validation_barred_reason.is_some()
}

fn expected_form_validation_block_reasons(control: &BrowserFormControl) -> Vec<String> {
    let mut reasons = Vec::new();
    if let Some(reason) = &control.validation_barred_reason {
        reasons.push(format!("validation-barred:{reason}"));
    }
    if control.disabled
        && !reasons
            .iter()
            .any(|reason| reason == "validation-barred:disabled")
    {
        reasons.push("disabled".to_string());
    }
    if control.readonly
        && !reasons
            .iter()
            .any(|reason| reason == "validation-barred:readonly")
    {
        reasons.push("readonly".to_string());
    }
    reasons
}

fn expected_form_validation_kind(
    control: &BrowserFormControl,
    form_novalidate: bool,
    submitter_novalidate_ids: &[String],
) -> String {
    if control.validation_barred_reason.is_some() {
        "barred-control".to_string()
    } else if form_novalidate {
        "form-novalidate-candidate".to_string()
    } else if !submitter_novalidate_ids.is_empty() {
        "submitter-novalidate-candidate".to_string()
    } else if control.required {
        "required-candidate".to_string()
    } else if !control.validation_attributes.is_empty() {
        "constraint-candidate".to_string()
    } else if control.will_validate {
        "validation-candidate".to_string()
    } else {
        "validation-metadata".to_string()
    }
}

fn expected_form_validation_element(control: &BrowserFormControl) -> String {
    match control.control_type.as_str() {
        "button" | "checkbox" | "color" | "date" | "datetime-local" | "email" | "file"
        | "hidden" | "image" | "month" | "number" | "password" | "radio" | "range" | "reset"
        | "search" | "submit" | "tel" | "text" | "time" | "url" | "week" => "input".to_string(),
        other => other.to_string(),
    }
}

fn expected_form_submitter_ids(form: &BrowserForm) -> Vec<String> {
    form.submitters
        .iter()
        .filter_map(|submitter| submitter.id.clone())
        .collect()
}

fn expected_form_submitter_novalidate_ids(form: &BrowserForm) -> Vec<String> {
    form.submitters
        .iter()
        .filter(|submitter| submitter.novalidate)
        .filter_map(|submitter| submitter.id.clone())
        .collect()
}

fn expected_tokens_from_raw(tokens: Vec<String>, raw: Option<&str>) -> Vec<String> {
    if !tokens.is_empty() {
        return tokens;
    }
    raw.map(split_html_test_tokens).unwrap_or_default()
}

fn split_html_test_tokens(raw: &str) -> Vec<String> {
    raw.split_ascii_whitespace()
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

impl ExpectedAnchor {
    fn into_browser_anchor(self) -> BrowserAnchor {
        BrowserAnchor {
            id: self.id,
            name: self.name,
            text: self.text,
        }
    }
}

impl ExpectedAnchorDescriptor {
    fn into_browser_anchor_descriptor(self) -> BrowserAnchorDescriptor {
        BrowserAnchorDescriptor {
            anchor_index: self.anchor_index,
            id: self.id,
            name: self.name,
            text: self.text,
            fragment_targets: self.fragment_targets,
            anchor_kind: self.anchor_kind,
            duplicate_target: self.duplicate_target,
            anchor_blocked: self.anchor_blocked,
            anchor_block_reasons: self.anchor_block_reasons,
        }
    }
}

impl ExpectedHeading {
    fn into_browser_heading(self) -> BrowserHeading {
        BrowserHeading {
            level: self.level,
            text: self.text,
        }
    }
}

impl ExpectedHeadingDescriptor {
    fn into_browser_heading_descriptor(self) -> BrowserHeadingDescriptor {
        BrowserHeadingDescriptor {
            heading_index: self.heading_index,
            level: self.level,
            text: self.text,
            previous_level: self.previous_level,
            outline_kind: self.outline_kind,
            skipped_level: self.skipped_level,
            heading_blocked: self.heading_blocked,
            heading_block_reasons: self.heading_block_reasons,
        }
    }
}

impl ExpectedTextSemantic {
    fn into_browser_text_semantic(self) -> BrowserTextSemantic {
        BrowserTextSemantic {
            element: self.element,
            id: self.id,
            title: self.title,
            role: self.role,
            text: self.text,
            lang: self.lang,
            dir: self.dir,
            quote_cite: self.quote_cite,
            resolved_quote_cite: self.resolved_quote_cite,
            data_value: self.data_value,
            datetime: self.datetime,
            edit_cite: self.edit_cite,
            resolved_edit_cite: self.resolved_edit_cite,
            edit_datetime: self.edit_datetime,
            ruby_kind: self.ruby_kind,
            bidi_kind: self.bidi_kind,
            phrase_kind: self.phrase_kind,
        }
    }
}

impl ExpectedTextSemanticDescriptor {
    fn into_browser_text_semantic_descriptor(self) -> BrowserTextSemanticDescriptor {
        BrowserTextSemanticDescriptor {
            semantic_index: self.semantic_index,
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            semantic_kind: self.semantic_kind,
            title: self.title,
            lang: self.lang,
            dir: self.dir,
            quote_cite: self.quote_cite,
            resolved_quote_cite: self.resolved_quote_cite,
            data_value: self.data_value,
            datetime: self.datetime,
            edit_cite: self.edit_cite,
            resolved_edit_cite: self.resolved_edit_cite,
            edit_datetime: self.edit_datetime,
            ruby_kind: self.ruby_kind,
            bidi_kind: self.bidi_kind,
            phrase_kind: self.phrase_kind,
            semantic_blocked: self.semantic_blocked,
            semantic_block_reasons: self.semantic_block_reasons,
        }
    }
}

impl ExpectedTextFlowDescriptor {
    fn into_browser_text_flow_descriptor(self) -> BrowserTextFlowDescriptor {
        BrowserTextFlowDescriptor {
            flow_index: self.flow_index,
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            flow_kind: self.flow_kind,
            text_flow: self.text_flow,
            list_kind: self.list_kind,
            list_start: self.list_start,
            list_marker_type: self.list_marker_type,
            list_reversed: self.list_reversed,
            list_item_value: self.list_item_value,
            list_item_count: self.list_item_count,
            description_list_kind: self.description_list_kind,
            term_kind: self.term_kind,
            term_count: self.term_count,
            description_count: self.description_count,
            quote_cite: self.quote_cite,
            resolved_quote_cite: self.resolved_quote_cite,
            flow_blocked: self.flow_blocked,
            flow_block_reasons: self.flow_block_reasons,
        }
    }
}

impl ExpectedNavigationGroup {
    fn into_browser_navigation_group(self) -> BrowserNavigationGroup {
        BrowserNavigationGroup {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            landmark_kind: self.landmark_kind,
            list_kind: self.list_kind,
            item_count: self.item_count,
            list_start: self.list_start,
            list_marker_type: self.list_marker_type,
            list_reversed: self.list_reversed,
        }
    }
}

impl ExpectedNavigationGroupDescriptor {
    fn into_browser_navigation_group_descriptor(self) -> BrowserNavigationGroupDescriptor {
        BrowserNavigationGroupDescriptor {
            group_index: self.group_index,
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            group_kind: self.group_kind,
            landmark_kind: self.landmark_kind,
            list_kind: self.list_kind,
            item_count: self.item_count,
            list_start: self.list_start,
            list_marker_type: self.list_marker_type,
            list_reversed: self.list_reversed,
            navigation_blocked: self.navigation_blocked,
            navigation_block_reasons: self.navigation_block_reasons,
        }
    }
}

impl ExpectedSectionLandmark {
    fn into_browser_section_landmark(self) -> BrowserSectionLandmark {
        BrowserSectionLandmark {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            text: self.text,
            accessible_name: self.accessible_name,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            section_kind: self.section_kind,
            landmark_kind: self.landmark_kind,
            heading_level: self.heading_level,
            heading_text: self.heading_text,
        }
    }
}

impl ExpectedSectionLandmarkDescriptor {
    fn into_browser_section_landmark_descriptor(self) -> BrowserSectionLandmarkDescriptor {
        BrowserSectionLandmarkDescriptor {
            landmark_index: self.landmark_index,
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            text: self.text,
            accessible_name: self.accessible_name,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            section_kind: self.section_kind,
            landmark_kind: self.landmark_kind,
            heading_level: self.heading_level,
            heading_text: self.heading_text,
            outline_kind: self.outline_kind,
            landmark_blocked: self.landmark_blocked,
            landmark_block_reasons: self.landmark_block_reasons,
        }
    }
}

impl ExpectedCommandElement {
    fn into_browser_command_element(self) -> BrowserCommandElement {
        BrowserCommandElement {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            command_kind: self.command_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            href: self.href,
            resolved_href: self.resolved_href,
            target: self.target,
            effective_target: self.effective_target,
            control_type: self.control_type,
            form_owner: self.form_owner,
            form_action: self.form_action,
            resolved_form_action: self.resolved_form_action,
            form_method: self.form_method,
            form_target: self.form_target,
            form_novalidate: self.form_novalidate,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            aria_controls: self.aria_controls,
            aria_expanded: self.aria_expanded,
            aria_haspopup: self.aria_haspopup,
            aria_pressed: self.aria_pressed,
            aria_current: self.aria_current,
            aria_disabled: self.aria_disabled,
            tabindex: self.tabindex,
            accesskey: self.accesskey,
            event_handlers: self.event_handlers,
            focusable: self.focusable,
            disabled: self.disabled,
        }
    }
}

impl ExpectedActivationDescriptor {
    fn into_browser_activation_descriptor(self) -> BrowserActivationDescriptor {
        BrowserActivationDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            command_kind: self.command_kind,
            activation_kind: self.activation_kind,
            target_id: self.target_id,
            target_kind: self.target_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            disabled: self.disabled,
            focusable: self.focusable,
            tabindex: self.tabindex,
            accesskey: self.accesskey,
            event_handlers: self.event_handlers,
            handler_count: self.handler_count,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            aria_controls: self.aria_controls,
            aria_expanded: self.aria_expanded,
            aria_haspopup: self.aria_haspopup,
            aria_pressed: self.aria_pressed,
            aria_current: self.aria_current,
            aria_disabled: self.aria_disabled,
            control_type: self.control_type,
            href: self.href,
            resolved_href: self.resolved_href,
            effective_target: self.effective_target,
            form_owner: self.form_owner,
            form_action: self.form_action,
            resolved_form_action: self.resolved_form_action,
            form_method: self.form_method,
            form_target: self.form_target,
            form_novalidate: self.form_novalidate,
        }
    }
}

impl ExpectedPopover {
    fn into_browser_popover(self) -> BrowserPopover {
        BrowserPopover {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            popover: self.popover,
            invokers: self
                .invokers
                .into_iter()
                .map(ExpectedPopoverInvoker::into_browser_popover_invoker)
                .collect(),
        }
    }
}

impl ExpectedPopoverInvoker {
    fn into_browser_popover_invoker(self) -> BrowserPopoverInvoker {
        BrowserPopoverInvoker {
            element: self.element,
            id: self.id,
            text: self.text,
            accessible_name: self.accessible_name,
            command_kind: self.command_kind,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            aria_controls: self.aria_controls,
            aria_expanded: self.aria_expanded,
            focusable: self.focusable,
        }
    }
}

impl ExpectedPopoverDescriptor {
    fn into_browser_popover_descriptor(self) -> BrowserPopoverDescriptor {
        BrowserPopoverDescriptor {
            popover_index: self.popover_index,
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            popover_mode: self.popover_mode,
            invoker_count: self.invoker_count,
            invoker_ids: self.invoker_ids,
            invoker_actions: self.invoker_actions,
            invoker_aria_expanded: self.invoker_aria_expanded,
            focusable_invoker_count: self.focusable_invoker_count,
            popover_blocked: self.popover_blocked,
            popover_block_reasons: self.popover_block_reasons,
        }
    }
}

impl ExpectedAriaCollection {
    fn into_browser_aria_collection(self) -> BrowserAriaCollection {
        BrowserAriaCollection {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_orientation: self.aria_orientation,
            aria_multiselectable: self.aria_multiselectable,
            aria_activedescendant: self.aria_activedescendant,
            aria_owns: self.aria_owns,
            item_count: self.item_count,
            selected_item_count: self.selected_item_count,
            checked_item_count: self.checked_item_count,
            current_item_count: self.current_item_count,
            disabled_item_count: self.disabled_item_count,
            item_roles: self.item_roles,
            selection_mode: self.selection_mode,
            active_descendant_matches_item: self.active_descendant_matches_item,
            items: self
                .items
                .into_iter()
                .map(ExpectedAriaCollectionItem::into_browser_aria_collection_item)
                .collect(),
        }
    }
}

impl ExpectedAriaCollectionItem {
    fn into_browser_aria_collection_item(self) -> BrowserAriaCollectionItem {
        BrowserAriaCollectionItem {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            aria_selected: self.aria_selected,
            aria_checked: self.aria_checked,
            aria_current: self.aria_current,
            aria_disabled: self.aria_disabled,
            aria_expanded: self.aria_expanded,
            aria_level: self.aria_level,
            aria_posinset: self.aria_posinset,
            aria_setsize: self.aria_setsize,
            aria_rowindex: self.aria_rowindex,
            aria_colindex: self.aria_colindex,
            aria_controls: self.aria_controls,
        }
    }
}

impl ExpectedAriaCollectionDescriptor {
    fn into_browser_aria_collection_descriptor(self) -> BrowserAriaCollectionDescriptor {
        BrowserAriaCollectionDescriptor {
            collection_index: self.collection_index,
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            collection_kind: self.collection_kind,
            aria_orientation: self.aria_orientation,
            aria_multiselectable: self.aria_multiselectable,
            aria_activedescendant: self.aria_activedescendant,
            aria_owns: self.aria_owns,
            item_count: self.item_count,
            item_roles: self.item_roles,
            selected_item_count: self.selected_item_count,
            checked_item_count: self.checked_item_count,
            current_item_count: self.current_item_count,
            disabled_item_count: self.disabled_item_count,
            selection_mode: self.selection_mode,
            active_descendant_matches_item: self.active_descendant_matches_item,
            collection_blocked: self.collection_blocked,
            collection_block_reasons: self.collection_block_reasons,
        }
    }
}

impl ExpectedAriaRange {
    fn into_browser_aria_range(self) -> BrowserAriaRange {
        BrowserAriaRange {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_valuenow: self.aria_valuenow,
            aria_valuemin: self.aria_valuemin,
            aria_valuemax: self.aria_valuemax,
            aria_valuetext: self.aria_valuetext,
            aria_orientation: self.aria_orientation,
            aria_disabled: self.aria_disabled,
            aria_readonly: self.aria_readonly,
            aria_required: self.aria_required,
            tabindex: self.tabindex,
            text_value: self.text_value,
            value_attribute_names: self.value_attribute_names,
            value_attribute_count: self.value_attribute_count,
            range_value_complete: self.range_value_complete,
            focusable: self.focusable,
            range_blocked: self.range_blocked,
            range_block_reasons: self.range_block_reasons,
        }
    }
}

impl ExpectedAriaRelationDescriptor {
    fn into_browser_aria_relation_descriptor(self) -> BrowserAriaRelationDescriptor {
        BrowserAriaRelationDescriptor {
            element: self.element,
            id: self.id,
            text: self.text,
            aria_details: self.aria_details,
            details_text: self.details_text,
            aria_errormessage: self.aria_errormessage,
            errormessage_text: self.errormessage_text,
            aria_flowto: self.aria_flowto,
            flowto_text: self.flowto_text,
            relation_attribute_names: self.relation_attribute_names,
            relation_attribute_count: self.relation_attribute_count,
            relation_target_count: self.relation_target_count,
            unresolved_relation_targets: self.unresolved_relation_targets,
            relation_blocked: self.relation_blocked,
            relation_block_reasons: self.relation_block_reasons,
        }
    }
}

impl ExpectedAriaNameDescriptor {
    fn into_browser_aria_name_descriptor(self) -> BrowserAriaNameDescriptor {
        BrowserAriaNameDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            labelledby_text: self.labelledby_text,
            name_source: self.name_source,
            name_attribute_names: self.name_attribute_names,
            name_attribute_count: self.name_attribute_count,
            label_target_count: self.label_target_count,
            unresolved_label_targets: self.unresolved_label_targets,
            name_blocked: self.name_blocked,
            name_block_reasons: self.name_block_reasons,
        }
    }
}

impl ExpectedAriaDescriptionDescriptor {
    fn into_browser_aria_description_descriptor(self) -> BrowserAriaDescriptionDescriptor {
        BrowserAriaDescriptionDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_description: self.aria_description,
            aria_describedby: self.aria_describedby,
            describedby_text: self.describedby_text,
            description_source: self.description_source,
            description_attribute_names: self.description_attribute_names,
            description_attribute_count: self.description_attribute_count,
            description_target_count: self.description_target_count,
            unresolved_description_targets: self.unresolved_description_targets,
            description_blocked: self.description_blocked,
            description_block_reasons: self.description_block_reasons,
        }
    }
}

impl ExpectedAriaLiveRegion {
    fn into_browser_aria_live_region(self) -> BrowserAriaLiveRegion {
        BrowserAriaLiveRegion {
            element: self.element,
            id: self.id,
            role: self.role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_live: self.aria_live,
            aria_busy: self.aria_busy,
            aria_atomic: self.aria_atomic,
            aria_relevant: self.aria_relevant,
            aria_hidden: self.aria_hidden,
            update_kind: self.update_kind,
            live_attribute_names: self.live_attribute_names,
            live_attribute_count: self.live_attribute_count,
            assertive_update: self.assertive_update,
            live_region_blocked: self.live_region_blocked,
            live_region_block_reasons: self.live_region_block_reasons,
        }
    }
}

impl ExpectedLink {
    fn into_browser_link(self) -> BrowserLink {
        BrowserLink {
            element: self.element,
            id: self.id,
            href: self.href,
            resolved_href: self.resolved_href,
            name: self.name,
            target: self.target,
            effective_target: self.effective_target,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            rel_external: self.rel_external,
            rel_nofollow: self.rel_nofollow,
            rel_noopener: self.rel_noopener,
            rel_noreferrer: self.rel_noreferrer,
            title: self.title,
            download: self.download,
            ping: self.ping,
            resolved_ping: self.resolved_ping,
            attributionsrc: self.attributionsrc,
            resolved_attributionsrc: self.resolved_attributionsrc,
            hreflang: self.hreflang,
            type_hint: self.type_hint,
            referrerpolicy: self.referrerpolicy,
            text: self.text,
        }
    }
}

fn default_browser_link_element() -> String {
    "a".to_string()
}

fn default_track_kind() -> String {
    "subtitles".to_string()
}

fn default_image_map_area_shape() -> String {
    "rect".to_string()
}

impl ExpectedImage {
    fn into_browser_image(self) -> BrowserImage {
        BrowserImage {
            src: self.src,
            resolved_src: self.resolved_src,
            alt: self.alt,
            width: self.width,
            height: self.height,
            srcset: self.srcset,
            resolved_srcset: self.resolved_srcset,
            sizes: self.sizes,
            loading: self.loading,
            decoding: self.decoding,
            fetchpriority: self.fetchpriority,
            crossorigin: self.crossorigin,
            referrerpolicy: self.referrerpolicy,
            usemap: self.usemap,
            ismap: self.ismap,
            sources: self
                .sources
                .into_iter()
                .map(ExpectedImageSource::into_browser_image_source)
                .collect(),
        }
    }
}

impl ExpectedImageSource {
    fn into_browser_image_source(self) -> BrowserImageSource {
        BrowserImageSource {
            srcset: self.srcset,
            resolved_srcset: self.resolved_srcset,
            sizes: self.sizes,
            media: self.media,
            type_hint: self.type_hint,
        }
    }
}

impl ExpectedImageCandidateDescriptor {
    fn into_browser_image_candidate_descriptor(self) -> BrowserImageCandidateDescriptor {
        BrowserImageCandidateDescriptor {
            src: self.src,
            resolved_src: self.resolved_src,
            srcset: self.srcset,
            resolved_srcset: self.resolved_srcset,
            sizes: self.sizes,
            alt: self.alt,
            width: self.width,
            height: self.height,
            loading: self.loading,
            decoding: self.decoding,
            fetchpriority: self.fetchpriority,
            crossorigin: self.crossorigin,
            referrerpolicy: self.referrerpolicy,
            usemap: self.usemap,
            ismap: self.ismap,
            has_alt: self.has_alt,
            source_count: self.source_count,
            source_srcset_count: self.source_srcset_count,
            candidate_count: self.candidate_count,
            source_type_hints: self.source_type_hints,
            source_media: self.source_media,
            sources: self
                .sources
                .into_iter()
                .map(ExpectedImageSource::into_browser_image_source)
                .collect(),
        }
    }
}

impl ExpectedImageMap {
    fn into_browser_image_map(self) -> BrowserImageMap {
        BrowserImageMap {
            id: self.id,
            name: self.name,
            areas: self
                .areas
                .into_iter()
                .map(ExpectedImageMapArea::into_browser_image_map_area)
                .collect(),
        }
    }
}

impl ExpectedImageMapArea {
    fn into_browser_image_map_area(self) -> BrowserImageMapArea {
        BrowserImageMapArea {
            id: self.id,
            shape: self.shape,
            coords: self.coords,
            href: self.href,
            resolved_href: self.resolved_href,
            alt: self.alt,
            target: self.target,
            effective_target: self.effective_target,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            rel_external: self.rel_external,
            rel_nofollow: self.rel_nofollow,
            rel_noopener: self.rel_noopener,
            rel_noreferrer: self.rel_noreferrer,
            ping: self.ping,
            resolved_ping: self.resolved_ping,
            attributionsrc: self.attributionsrc,
            resolved_attributionsrc: self.resolved_attributionsrc,
            download: self.download,
            hreflang: self.hreflang,
            referrerpolicy: self.referrerpolicy,
        }
    }
}

impl ExpectedImageMapDescriptor {
    fn into_browser_image_map_descriptor(self) -> BrowserImageMapDescriptor {
        BrowserImageMapDescriptor {
            map_index: self.map_index,
            id: self.id,
            name: self.name,
            referenced_image_sources: self.referenced_image_sources,
            area_count: self.area_count,
            navigable_area_count: self.navigable_area_count,
            area_shapes: self.area_shapes,
            missing_alt_area_count: self.missing_alt_area_count,
            missing_href_area_count: self.missing_href_area_count,
            missing_coords_area_count: self.missing_coords_area_count,
            default_shape_area_count: self.default_shape_area_count,
            ping_area_count: self.ping_area_count,
            attribution_area_count: self.attribution_area_count,
            map_blocked: self.map_blocked,
            map_block_reasons: self.map_block_reasons,
        }
    }
}

impl ExpectedMedia {
    fn into_browser_media(self) -> BrowserMedia {
        BrowserMedia {
            kind: self.kind,
            src: self.src,
            resolved_src: self.resolved_src,
            poster: self.poster,
            resolved_poster: self.resolved_poster,
            width: self.width,
            height: self.height,
            controls: self.controls,
            autoplay: self.autoplay,
            loop_media: self.loop_media,
            muted: self.muted,
            playsinline: self.playsinline,
            preload: self.preload,
            crossorigin: self.crossorigin,
            controlslist: self.controlslist,
            controlslist_tokens: self.controlslist_tokens,
            disableremoteplayback: self.disableremoteplayback,
            disablepictureinpicture: self.disablepictureinpicture,
            sources: self
                .sources
                .into_iter()
                .map(ExpectedMediaSource::into_browser_media_source)
                .collect(),
            tracks: self
                .tracks
                .into_iter()
                .map(ExpectedMediaTrack::into_browser_media_track)
                .collect(),
        }
    }
}

impl ExpectedMediaSource {
    fn into_browser_media_source(self) -> BrowserMediaSource {
        BrowserMediaSource {
            src: self.src,
            resolved_src: self.resolved_src,
            type_hint: self.type_hint,
            media: self.media,
        }
    }
}

impl ExpectedMediaTrack {
    fn into_browser_media_track(self) -> BrowserMediaTrack {
        BrowserMediaTrack {
            kind: self.kind,
            src: self.src,
            resolved_src: self.resolved_src,
            srclang: self.srclang,
            label: self.label,
            default_track: self.default_track,
        }
    }
}

impl ExpectedMediaPlaybackDescriptor {
    fn into_browser_media_playback_descriptor(self) -> BrowserMediaPlaybackDescriptor {
        BrowserMediaPlaybackDescriptor {
            kind: self.kind,
            src: self.src,
            resolved_src: self.resolved_src,
            poster: self.poster,
            resolved_poster: self.resolved_poster,
            width: self.width,
            height: self.height,
            controls: self.controls,
            autoplay: self.autoplay,
            loop_media: self.loop_media,
            muted: self.muted,
            playsinline: self.playsinline,
            preload: self.preload,
            crossorigin: self.crossorigin,
            controlslist: self.controlslist,
            controlslist_tokens: self.controlslist_tokens,
            disableremoteplayback: self.disableremoteplayback,
            disablepictureinpicture: self.disablepictureinpicture,
            source_count: self.source_count,
            sources: self
                .sources
                .into_iter()
                .map(ExpectedMediaSource::into_browser_media_source)
                .collect(),
            track_count: self.track_count,
            default_track_count: self.default_track_count,
            tracks: self
                .tracks
                .into_iter()
                .map(ExpectedMediaTrack::into_browser_media_track)
                .collect(),
        }
    }
}

impl ExpectedMediaResourceDescriptor {
    fn into_browser_media_resource_descriptor(self) -> BrowserMediaResourceDescriptor {
        BrowserMediaResourceDescriptor {
            media_index: self.media_index,
            media_kind: self.media_kind,
            element: self.element,
            resource_kind: self.resource_kind,
            src: self.src,
            resolved_src: self.resolved_src,
            type_hint: self.type_hint,
            media: self.media,
            track_kind: self.track_kind,
            srclang: self.srclang,
            label: self.label,
            default_track: self.default_track,
            candidate_kind: self.candidate_kind,
            media_resource_blocked: self.media_resource_blocked,
            media_resource_block_reasons: self.media_resource_block_reasons,
        }
    }
}

impl ExpectedEmbeddedContext {
    fn into_browser_embedded_context(self) -> BrowserEmbeddedContext {
        BrowserEmbeddedContext {
            element: self.element,
            url: self.url,
            resolved_url: self.resolved_url,
            browsing_context_name: self.browsing_context_name,
            title: self.title,
            type_hint: self.type_hint,
            width: self.width,
            height: self.height,
            loading: self.loading,
            fetchpriority: self.fetchpriority,
            csp: self.csp,
            sandbox: self.sandbox,
            allow: self.allow,
            allowfullscreen: self.allowfullscreen,
            referrerpolicy: self.referrerpolicy,
            srcdoc: self.srcdoc,
            credentialless: self.credentialless,
            fallback_text: self.fallback_text,
        }
    }
}

impl ExpectedEmbeddedPolicyDescriptor {
    fn into_browser_embedded_policy_descriptor(self) -> BrowserEmbeddedPolicyDescriptor {
        BrowserEmbeddedPolicyDescriptor {
            element: self.element,
            resource_kind: self.resource_kind,
            url: self.url,
            resolved_url: self.resolved_url,
            browsing_context_name: self.browsing_context_name,
            title: self.title,
            type_hint: self.type_hint,
            width: self.width,
            height: self.height,
            loading: self.loading,
            fetchpriority: self.fetchpriority,
            csp: self.csp,
            sandbox: self.sandbox,
            sandbox_token_count: self.sandbox_token_count,
            allow: self.allow,
            allow_tokens: self.allow_tokens,
            allow_token_count: self.allow_token_count,
            allowfullscreen: self.allowfullscreen,
            fullscreen_allowed: self.fullscreen_allowed,
            referrerpolicy: self.referrerpolicy,
            srcdoc: self.srcdoc,
            has_srcdoc: self.has_srcdoc,
            credentialless: self.credentialless,
            fallback_text: self.fallback_text,
        }
    }
}

impl ExpectedInteractiveElement {
    fn into_browser_interactive_element(self) -> BrowserInteractiveElement {
        BrowserInteractiveElement {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_controls: self.aria_controls,
            aria_owns: self.aria_owns,
            aria_activedescendant: self.aria_activedescendant,
            aria_current: self.aria_current,
            aria_expanded: self.aria_expanded,
            aria_haspopup: self.aria_haspopup,
            aria_modal: self.aria_modal,
            aria_pressed: self.aria_pressed,
            aria_selected: self.aria_selected,
            aria_invalid: self.aria_invalid,
            aria_live: self.aria_live,
            aria_busy: self.aria_busy,
            aria_disabled: self.aria_disabled,
            aria_required: self.aria_required,
            aria_keyshortcuts: self.aria_keyshortcuts,
            aria_hidden: self.aria_hidden,
            hidden: self.hidden,
            inert: self.inert,
            open: self.open,
            tabindex: self.tabindex,
            accesskey: self.accesskey,
            event_handlers: self.event_handlers,
            focusable: self.focusable,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            draggable: self.draggable,
            draggable_state: self.draggable_state,
            spellcheck: self.spellcheck,
            translate: self.translate,
            popover: self.popover,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            command: self.command,
            command_for: self.command_for,
            disabled: self.disabled,
        }
    }
}

impl ExpectedFocusNavigationDescriptor {
    fn into_browser_focus_navigation_descriptor(self) -> BrowserFocusNavigationDescriptor {
        BrowserFocusNavigationDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            focus_kind: self.focus_kind,
            focusable: self.focusable,
            sequential_focus: self.sequential_focus,
            programmatic_focus: self.programmatic_focus,
            focus_blocked: self.focus_blocked,
            focus_block_reasons: self.focus_block_reasons,
            tabindex: self.tabindex,
            tabindex_order: self.tabindex_order,
            accesskey: self.accesskey,
            event_handlers: self.event_handlers,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            aria_controls: self.aria_controls,
            aria_activedescendant: self.aria_activedescendant,
            aria_expanded: self.aria_expanded,
            aria_haspopup: self.aria_haspopup,
            aria_disabled: self.aria_disabled,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            text: self.text,
        }
    }
}

impl ExpectedKeyboardInteractionDescriptor {
    fn into_browser_keyboard_interaction_descriptor(self) -> BrowserKeyboardInteractionDescriptor {
        BrowserKeyboardInteractionDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            keyboard_kind: self.keyboard_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            focusable: self.focusable,
            sequential_focus: self.sequential_focus,
            programmatic_focus: self.programmatic_focus,
            tabindex: self.tabindex,
            tabindex_order: self.tabindex_order,
            accesskey: self.accesskey,
            aria_keyshortcuts: self.aria_keyshortcuts,
            keyboard_handlers: self.keyboard_handlers,
            handler_count: self.handler_count,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            aria_controls: self.aria_controls,
            aria_activedescendant: self.aria_activedescendant,
            aria_expanded: self.aria_expanded,
            aria_haspopup: self.aria_haspopup,
            aria_disabled: self.aria_disabled,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            keyboard_blocked: self.keyboard_blocked,
            keyboard_block_reasons: self.keyboard_block_reasons,
        }
    }
}

impl ExpectedInputPlanningDescriptor {
    fn into_browser_input_planning_descriptor(self) -> BrowserInputPlanningDescriptor {
        BrowserInputPlanningDescriptor {
            element: self.element,
            id: self.id,
            input_kind: self.input_kind,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            labels: self.labels,
            placeholder: self.placeholder,
            value: self.value,
            editing_mode: self.editing_mode,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            autocapitalize: self.autocapitalize,
            enterkeyhint: self.enterkeyhint,
            dirname: self.dirname,
            spellcheck: self.spellcheck,
            autocorrect: self.autocorrect,
            inputmode: self.inputmode,
            pattern: self.pattern,
            min: self.min,
            max: self.max,
            step: self.step,
            minlength: self.minlength,
            maxlength: self.maxlength,
            size: self.size,
            rows: self.rows,
            cols: self.cols,
            wrap: self.wrap,
            list: self.list,
            datalist_options: self.datalist_options,
            focusable: self.focusable,
            input_handlers: self.input_handlers,
            disabled: self.disabled,
            required: self.required,
            readonly: self.readonly,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            input_blocked: self.input_blocked,
            input_block_reasons: self.input_block_reasons,
        }
    }
}

impl ExpectedDragDropDescriptor {
    fn into_browser_drag_drop_descriptor(self) -> BrowserDragDropDescriptor {
        BrowserDragDropDescriptor {
            element: self.element,
            id: self.id,
            classes: self.classes,
            role: self.role,
            authored_role: self.authored_role,
            drag_kind: self.drag_kind,
            text: self.text,
            draggable: self.draggable,
            draggable_state: self.draggable_state,
            drag_source: self.drag_source,
            drop_target: self.drop_target,
            drag_handlers: self.drag_handlers,
            drop_handlers: self.drop_handlers,
            pointer_handlers: self.pointer_handlers,
            handler_count: self.handler_count,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            drag_blocked: self.drag_blocked,
            drag_block_reasons: self.drag_block_reasons,
        }
    }
}

impl ExpectedClipboardInteractionDescriptor {
    fn into_browser_clipboard_interaction_descriptor(
        self,
    ) -> BrowserClipboardInteractionDescriptor {
        BrowserClipboardInteractionDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            clipboard_kind: self.clipboard_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            value: self.value,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            spellcheck: self.spellcheck,
            clipboard_handlers: self.clipboard_handlers,
            copy_handlers: self.copy_handlers,
            cut_handlers: self.cut_handlers,
            paste_handlers: self.paste_handlers,
            input_handlers: self.input_handlers,
            handler_count: self.handler_count,
            focusable: self.focusable,
            readonly: self.readonly,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            clipboard_blocked: self.clipboard_blocked,
            clipboard_block_reasons: self.clipboard_block_reasons,
        }
    }
}

impl ExpectedSelectionInteractionDescriptor {
    fn into_browser_selection_interaction_descriptor(
        self,
    ) -> BrowserSelectionInteractionDescriptor {
        BrowserSelectionInteractionDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            selection_kind: self.selection_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            value: self.value,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            spellcheck: self.spellcheck,
            selection_handlers: self.selection_handlers,
            select_handlers: self.select_handlers,
            selection_change_handlers: self.selection_change_handlers,
            input_handlers: self.input_handlers,
            handler_count: self.handler_count,
            focusable: self.focusable,
            readonly: self.readonly,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            selection_blocked: self.selection_blocked,
            selection_block_reasons: self.selection_block_reasons,
        }
    }
}

impl ExpectedCompositionInteractionDescriptor {
    fn into_browser_composition_interaction_descriptor(
        self,
    ) -> BrowserCompositionInteractionDescriptor {
        BrowserCompositionInteractionDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            source: self.source,
            composition_kind: self.composition_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            value: self.value,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            spellcheck: self.spellcheck,
            inputmode: self.inputmode,
            enterkeyhint: self.enterkeyhint,
            composition_handlers: self.composition_handlers,
            composition_start_handlers: self.composition_start_handlers,
            composition_update_handlers: self.composition_update_handlers,
            composition_end_handlers: self.composition_end_handlers,
            beforeinput_handlers: self.beforeinput_handlers,
            input_handlers: self.input_handlers,
            handler_count: self.handler_count,
            focusable: self.focusable,
            readonly: self.readonly,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            composition_blocked: self.composition_blocked,
            composition_block_reasons: self.composition_block_reasons,
        }
    }
}

impl ExpectedPointerInteractionDescriptor {
    fn into_browser_pointer_interaction_descriptor(self) -> BrowserPointerInteractionDescriptor {
        BrowserPointerInteractionDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            pointer_kind: self.pointer_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            control_type: self.control_type,
            command: self.command,
            command_for: self.command_for,
            popover_target: self.popover_target,
            popover_target_action: self.popover_target_action,
            contenteditable: self.contenteditable,
            editing_mode: self.editing_mode,
            draggable: self.draggable,
            draggable_state: self.draggable_state,
            pointer_handlers: self.pointer_handlers,
            mouse_handlers: self.mouse_handlers,
            touch_handlers: self.touch_handlers,
            wheel_handlers: self.wheel_handlers,
            click_handlers: self.click_handlers,
            drag_handlers: self.drag_handlers,
            drop_handlers: self.drop_handlers,
            handler_count: self.handler_count,
            focusable: self.focusable,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            pointer_blocked: self.pointer_blocked,
            pointer_block_reasons: self.pointer_block_reasons,
        }
    }
}

impl ExpectedScrollInteractionDescriptor {
    fn into_browser_scroll_interaction_descriptor(self) -> BrowserScrollInteractionDescriptor {
        BrowserScrollInteractionDescriptor {
            element: self.element,
            id: self.id,
            role: self.role,
            authored_role: self.authored_role,
            source: self.source,
            scroll_kind: self.scroll_kind,
            text: self.text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_valuenow: self.aria_valuenow,
            aria_valuemin: self.aria_valuemin,
            aria_valuemax: self.aria_valuemax,
            aria_valuetext: self.aria_valuetext,
            aria_orientation: self.aria_orientation,
            aria_disabled: self.aria_disabled,
            aria_readonly: self.aria_readonly,
            tabindex: self.tabindex,
            scroll_handlers: self.scroll_handlers,
            wheel_handlers: self.wheel_handlers,
            touch_handlers: self.touch_handlers,
            handler_count: self.handler_count,
            focusable: self.focusable,
            disabled: self.disabled,
            hidden: self.hidden,
            inert: self.inert,
            aria_hidden: self.aria_hidden,
            scroll_blocked: self.scroll_blocked,
            scroll_block_reasons: self.scroll_block_reasons,
        }
    }
}

impl ExpectedDisclosure {
    fn into_browser_disclosure(self) -> BrowserDisclosure {
        BrowserDisclosure {
            element: self.element,
            id: self.id,
            name: self.name,
            text: self.text,
            summary_text: self.summary_text,
            open: self.open,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_modal: self.aria_modal,
            closedby: self.closedby,
        }
    }
}

impl ExpectedDisclosureStateDescriptor {
    fn into_browser_disclosure_state_descriptor(self) -> BrowserDisclosureStateDescriptor {
        BrowserDisclosureStateDescriptor {
            element: self.element,
            id: self.id,
            name: self.name,
            disclosure_kind: self.disclosure_kind,
            open: self.open,
            grouped: self.grouped,
            group_name: self.group_name,
            has_summary: self.has_summary,
            summary_text: self.summary_text,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            aria_label: self.aria_label,
            aria_labelledby: self.aria_labelledby,
            aria_describedby: self.aria_describedby,
            aria_modal: self.aria_modal,
            modal: self.modal,
            closedby: self.closedby,
            text: self.text,
            text_length: self.text_length,
        }
    }
}

impl ExpectedForm {
    fn into_browser_form(self) -> BrowserForm {
        BrowserForm {
            id: self.id,
            action: self.action,
            resolved_action: self.resolved_action,
            name: self.name,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            accept_charset: self.accept_charset,
            accept_charset_tokens: self.accept_charset_tokens,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            rel: self.rel,
            rel_tokens: self.rel_tokens,
            rel_external: self.rel_external,
            rel_nofollow: self.rel_nofollow,
            rel_noopener: self.rel_noopener,
            rel_noreferrer: self.rel_noreferrer,
            novalidate: self.novalidate,
            event_handlers: self.event_handlers,
            fieldsets: self
                .fieldsets
                .into_iter()
                .map(ExpectedFormFieldset::into_browser_form_fieldset)
                .collect(),
            labels: self
                .labels
                .into_iter()
                .map(ExpectedFormLabel::into_browser_form_label)
                .collect(),
            datalists: self
                .datalists
                .into_iter()
                .map(ExpectedFormDatalist::into_browser_form_datalist)
                .collect(),
            selects: self
                .selects
                .into_iter()
                .map(ExpectedFormSelect::into_browser_form_select)
                .collect(),
            outputs: self
                .outputs
                .into_iter()
                .map(ExpectedFormOutput::into_browser_form_output)
                .collect(),
            measurements: self
                .measurements
                .into_iter()
                .map(ExpectedFormMeasurement::into_browser_form_measurement)
                .collect(),
            object_controls: self
                .object_controls
                .into_iter()
                .map(ExpectedFormObject::into_browser_form_object)
                .collect(),
            successful_controls: self
                .successful_controls
                .into_iter()
                .map(ExpectedFormSuccessfulControl::into_browser_form_successful_control)
                .collect(),
            validation_controls: self
                .validation_controls
                .into_iter()
                .map(ExpectedFormValidationControl::into_browser_form_validation_control)
                .collect(),
            buttons: self
                .buttons
                .into_iter()
                .map(ExpectedFormButton::into_browser_form_button)
                .collect(),
            text_entries: self
                .text_entries
                .into_iter()
                .map(ExpectedFormTextEntry::into_browser_form_text_entry)
                .collect(),
            choice_controls: self
                .choice_controls
                .into_iter()
                .map(ExpectedFormChoiceControl::into_browser_form_choice_control)
                .collect(),
            file_controls: self
                .file_controls
                .into_iter()
                .map(ExpectedFormFileControl::into_browser_form_file_control)
                .collect(),
            hidden_controls: self
                .hidden_controls
                .into_iter()
                .map(ExpectedFormHiddenControl::into_browser_form_hidden_control)
                .collect(),
            image_controls: self
                .image_controls
                .into_iter()
                .map(ExpectedFormImageControl::into_browser_form_image_control)
                .collect(),
            controls: self
                .controls
                .into_iter()
                .map(ExpectedFormControl::into_browser_form_control)
                .collect(),
            submitters: self
                .submitters
                .into_iter()
                .map(ExpectedFormSubmitter::into_browser_form_submitter)
                .collect(),
        }
    }
}

impl ExpectedFormFieldset {
    fn into_browser_form_fieldset(self) -> BrowserFormFieldset {
        BrowserFormFieldset {
            id: self.id,
            form_owner: self.form_owner,
            legend: self.legend,
            disabled: self.disabled,
            control_ids: self.control_ids,
            control_names: self.control_names,
        }
    }
}

impl ExpectedFormLabel {
    fn into_browser_form_label(self) -> BrowserFormLabel {
        BrowserFormLabel {
            id: self.id,
            for_control: self.for_control,
            text: self.text,
            control_id: self.control_id,
            control_name: self.control_name,
            control_type: self.control_type,
            association: self.association,
        }
    }
}

impl ExpectedFormDatalist {
    fn into_browser_form_datalist(self) -> BrowserFormDatalist {
        BrowserFormDatalist {
            id: self.id,
            control_ids: self.control_ids,
            control_names: self.control_names,
            options: self
                .options
                .into_iter()
                .map(ExpectedDatalistOption::into_browser_datalist_option)
                .collect(),
        }
    }
}

impl ExpectedDatalistOption {
    fn into_browser_datalist_option(self) -> BrowserDatalistOption {
        BrowserDatalistOption {
            value: self.value,
            label: self.label,
            text: self.text,
            disabled: self.disabled,
        }
    }
}

impl ExpectedFormOutput {
    fn into_browser_form_output(self) -> BrowserFormOutput {
        BrowserFormOutput {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            for_tokens: self.for_tokens,
            for_control_ids: self.for_control_ids,
            for_control_names: self.for_control_names,
            for_control_types: self.for_control_types,
            value: self.value,
            disabled: self.disabled,
            will_validate: self.will_validate,
            validation_barred_reason: self.validation_barred_reason,
            text: self.text,
        }
    }
}

impl ExpectedFormMeasurement {
    fn into_browser_form_measurement(self) -> BrowserFormMeasurement {
        BrowserFormMeasurement {
            id: self.id,
            measurement_type: self.measurement_type,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            value: self.value,
            min: self.min,
            max: self.max,
            low: self.low,
            high: self.high,
            optimum: self.optimum,
            indeterminate: self.indeterminate,
            text: self.text,
        }
    }
}

impl ExpectedFormObject {
    fn into_browser_form_object(self) -> BrowserFormObject {
        BrowserFormObject {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            data: self.data,
            resolved_data: self.resolved_data,
            type_hint: self.type_hint,
            width: self.width,
            height: self.height,
            usemap: self.usemap,
            fallback_text: self.fallback_text,
            params: self
                .params
                .into_iter()
                .map(ExpectedFormObjectParam::into_browser_form_object_param)
                .collect(),
        }
    }
}

impl ExpectedFormObjectParam {
    fn into_browser_form_object_param(self) -> BrowserFormObjectParam {
        BrowserFormObjectParam {
            name: self.name,
            value: self.value,
        }
    }
}

impl ExpectedFormSuccessfulControl {
    fn into_browser_form_successful_control(self) -> BrowserFormSuccessfulControl {
        BrowserFormSuccessfulControl {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            submission_values: self.submission_values,
        }
    }
}

impl ExpectedFormValidationControl {
    fn into_browser_form_validation_control(self) -> BrowserFormValidationControl {
        BrowserFormValidationControl {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            will_validate: self.will_validate,
            required: self.required,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
        }
    }
}

impl ExpectedFormSelect {
    fn into_browser_form_select(self) -> BrowserFormSelect {
        BrowserFormSelect {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            disabled: self.disabled,
            required: self.required,
            multiple: self.multiple,
            size: self.size,
            value: self.value,
            selected_options: self.selected_options,
            options: self
                .options
                .into_iter()
                .map(ExpectedSelectOption::into_browser_select_option)
                .collect(),
            text: self.text,
        }
    }
}

impl ExpectedFormTextEntry {
    fn into_browser_form_text_entry(self) -> BrowserFormTextEntry {
        BrowserFormTextEntry {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            placeholder: self.placeholder,
            value: self.value,
            text: self.text,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            autocapitalize: self.autocapitalize,
            enterkeyhint: self.enterkeyhint,
            dirname: self.dirname,
            spellcheck: self.spellcheck,
            autocorrect: self.autocorrect,
            inputmode: self.inputmode,
            pattern: self.pattern,
            min: self.min,
            max: self.max,
            step: self.step,
            minlength: self.minlength,
            maxlength: self.maxlength,
            size: self.size,
            rows: self.rows,
            cols: self.cols,
            wrap: self.wrap,
            list: self.list,
            datalist_options: self.datalist_options,
            disabled: self.disabled,
            required: self.required,
            readonly: self.readonly,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
        }
    }
}

impl ExpectedFormChoiceControl {
    fn into_browser_form_choice_control(self) -> BrowserFormChoiceControl {
        BrowserFormChoiceControl {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            value: self.value,
            checked: self.checked,
            disabled: self.disabled,
            required: self.required,
            group_required: self.group_required,
            successful: self.successful,
            submission_values: self.submission_values,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
            group_name: self.group_name,
            group_checked_ids: self.group_checked_ids,
            group_checked_values: self.group_checked_values,
        }
    }
}

impl ExpectedFormFileControl {
    fn into_browser_form_file_control(self) -> BrowserFormFileControl {
        BrowserFormFileControl {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accept: self.accept,
            accept_tokens: self.accept_tokens,
            capture: self.capture,
            multiple: self.multiple,
            disabled: self.disabled,
            required: self.required,
            successful: self.successful,
            submission_values: self.submission_values,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
        }
    }
}

impl ExpectedFormHiddenControl {
    fn into_browser_form_hidden_control(self) -> BrowserFormHiddenControl {
        BrowserFormHiddenControl {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            value: self.value,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            disabled: self.disabled,
            successful: self.successful,
            submission_values: self.submission_values,
            will_validate: self.will_validate,
            validation_barred_reason: self.validation_barred_reason,
        }
    }
}

impl ExpectedFormImageControl {
    fn into_browser_form_image_control(self) -> BrowserFormImageControl {
        BrowserFormImageControl {
            id: self.id,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            src: self.src,
            resolved_src: self.resolved_src,
            alt: self.alt,
            width: self.width,
            height: self.height,
            disabled: self.disabled,
            autofocus: self.autofocus,
            submitter: self.submitter,
            action: self.action,
            resolved_action: self.resolved_action,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            novalidate: self.novalidate,
            value: self.value,
            coordinate_names: self.coordinate_names,
            will_validate: self.will_validate,
            validation_barred_reason: self.validation_barred_reason,
        }
    }
}

impl ExpectedFormControl {
    fn into_browser_form_control(self) -> BrowserFormControl {
        BrowserFormControl {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            labels: self.labels,
            accessible_name: self.accessible_name,
            accessible_description: self.accessible_description,
            placeholder: self.placeholder,
            autocomplete: self.autocomplete,
            autocomplete_tokens: self.autocomplete_tokens,
            autocapitalize: self.autocapitalize,
            enterkeyhint: self.enterkeyhint,
            dirname: self.dirname,
            spellcheck: self.spellcheck,
            autocorrect: self.autocorrect,
            accept: self.accept,
            accept_tokens: self.accept_tokens,
            capture: self.capture,
            src: self.src,
            resolved_src: self.resolved_src,
            alt: self.alt,
            width: self.width,
            height: self.height,
            inputmode: self.inputmode,
            pattern: self.pattern,
            min: self.min,
            max: self.max,
            step: self.step,
            minlength: self.minlength,
            maxlength: self.maxlength,
            size: self.size,
            rows: self.rows,
            cols: self.cols,
            wrap: self.wrap,
            list: self.list,
            datalist_options: self.datalist_options,
            output_for: self.output_for,
            form_action: self.form_action,
            resolved_form_action: self.resolved_form_action,
            form_enctype: self.form_enctype,
            form_method: self.form_method,
            form_target: self.form_target,
            form_novalidate: self.form_novalidate,
            value: self.value,
            successful: self.successful,
            submission_values: self.submission_values,
            autofocus: self.autofocus,
            disabled: self.disabled,
            required: self.required,
            readonly: self.readonly,
            will_validate: self.will_validate,
            validation_attributes: self.validation_attributes,
            validation_barred_reason: self.validation_barred_reason,
            checked: self.checked,
            multiple: self.multiple,
            selected_options: self.selected_options,
            option_items: self
                .option_items
                .into_iter()
                .map(ExpectedSelectOption::into_browser_select_option)
                .collect(),
            text: self.text,
            options: self.options,
        }
    }
}

impl ExpectedSelectOption {
    fn into_browser_select_option(self) -> BrowserSelectOption {
        BrowserSelectOption {
            value: self.value,
            label: self.label,
            text: self.text,
            selected: self.selected,
            disabled: self.disabled,
            group_label: self.group_label,
        }
    }
}

impl ExpectedFormSubmitter {
    fn into_browser_form_submitter(self) -> BrowserFormSubmitter {
        BrowserFormSubmitter {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            accessible_name: self.accessible_name,
            action: self.action,
            resolved_action: self.resolved_action,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            novalidate: self.novalidate,
            value: self.value,
        }
    }
}

impl ExpectedFormButton {
    fn into_browser_form_button(self) -> BrowserFormButton {
        BrowserFormButton {
            id: self.id,
            control_type: self.control_type,
            name: self.name,
            form_owner: self.form_owner,
            accessible_name: self.accessible_name,
            disabled: self.disabled,
            autofocus: self.autofocus,
            submitter: self.submitter,
            action: self.action,
            resolved_action: self.resolved_action,
            method: self.method,
            enctype: self.enctype,
            target: self.target,
            effective_target: self.effective_target,
            novalidate: self.novalidate,
            value: self.value,
            text: self.text,
            src: self.src,
            resolved_src: self.resolved_src,
            alt: self.alt,
            width: self.width,
            height: self.height,
        }
    }
}

impl ExpectedTable {
    fn into_browser_table(self) -> BrowserTable {
        BrowserTable {
            caption: self.caption,
            row_count: self.row_count,
            column_count: self.column_count,
            column_hint_count: self.column_hint_count,
            cell_count: self.cell_count,
            header_cell_count: self.header_cell_count,
        }
    }
}

impl ExpectedTableCell {
    fn into_browser_table_cell(self) -> BrowserTableCell {
        BrowserTableCell {
            table_index: self.table_index,
            table_id: self.table_id,
            table_caption: self.table_caption,
            section_kind: self.section_kind,
            row_index: self.row_index,
            column_index: self.column_index,
            element: self.element,
            id: self.id,
            text: self.text,
            accessible_name: self.accessible_name,
            header: self.header,
            scope: self.scope,
            headers: self.headers,
            abbr: self.abbr,
            rowspan: self.rowspan,
            colspan: self.colspan,
        }
    }
}

impl ExpectedTableStructureDescriptor {
    fn into_browser_table_structure_descriptor(self) -> BrowserTableStructureDescriptor {
        BrowserTableStructureDescriptor {
            table_index: self.table_index,
            table_id: self.table_id,
            caption: self.caption,
            row_count: self.row_count,
            column_count: self.column_count,
            column_hint_count: self.column_hint_count,
            cell_count: self.cell_count,
            header_cell_count: self.header_cell_count,
            section_kinds: self.section_kinds,
            header_scopes: self.header_scopes,
            header_ids: self.header_ids,
            cells_with_headers_count: self.cells_with_headers_count,
            spanning_cell_count: self.spanning_cell_count,
            table_blocked: self.table_blocked,
            table_block_reasons: self.table_block_reasons,
        }
    }
}
