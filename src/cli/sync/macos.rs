use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::mpsc;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2::Message;
use objc2_contacts::{
    CNContact, CNContactEmailAddressesKey, CNContactFamilyNameKey,
    CNContactGivenNameKey, CNContactIdentifierKey, CNContactImageDataKey,
    CNContactJobTitleKey, CNContactMiddleNameKey, CNContactNamePrefixKey,
    CNContactNameSuffixKey, CNContactNicknameKey, CNContactOrganizationNameKey,
    CNContactPhoneNumbersKey, CNContactPostalAddressesKey, CNContactBirthdayKey,
    CNContactStore, CNEntityType, CNAuthorizationStatus,
    CNPhoneNumber, CNPostalAddress, CNKeyDescriptor,
    CNMutableContact, CNSaveRequest,
};
use objc2_foundation::{NSArray, NSError, NSMutableCopying, NSString, NSPredicate};

use crate::cli::photo_utils;
use crate::db::Database;
use crate::models::{
    Address, AddressType, DateType, Email, EmailType, Person, PersonOrganization,
    Phone, PhoneType, SpecialDate,
};

/// Sync contacts from macOS Contacts app
pub fn run_sync_mac(db: &Database, dry_run: bool) -> Result<()> {
    println!("Syncing contacts from macOS Contacts...");

    let store = unsafe { CNContactStore::new() };

    // Check authorization
    let status = unsafe { CNContactStore::authorizationStatusForEntityType(CNEntityType::Contacts) };

    match status {
        CNAuthorizationStatus::Authorized => {}
        CNAuthorizationStatus::NotDetermined => {
            println!("Requesting access to Contacts...");
            request_authorization(&store)?;
        }
        CNAuthorizationStatus::Denied | CNAuthorizationStatus::Restricted => {
            return Err(anyhow!(
                "Access to Contacts denied.\n\n\
                To grant access:\n\
                1. Open System Settings > Privacy & Security > Contacts\n\
                2. Enable access for Terminal (or your terminal app)\n\
                3. Run this command again"
            ));
        }
        _ => {
            return Err(anyhow!("Unknown authorization status"));
        }
    }

    // Fetch all contacts using predicate
    let contacts = fetch_all_contacts(&store)?;
    let total = contacts.count();

    if total == 0 {
        println!("No contacts found in macOS Contacts.");
        return Ok(());
    }

    println!("Found {} contacts", total);

    if dry_run {
        println!("\n[DRY RUN] Would import the following contacts:\n");
    }

    let mut created = 0;
    let mut updated = 0;
    let mut skipped = 0;

    for i in 0..total {
        let contact = contacts.objectAtIndex(i);

        let apple_id = get_contact_identifier(&contact);

        if apple_id.is_empty() {
            skipped += 1;
            continue;
        }

        let given = get_contact_given_name(&contact);
        let family = get_contact_family_name(&contact);
        let org_name = get_contact_organization(&contact);

        // Skip contacts with no name and no organization
        if given.is_empty() && family.is_empty() && org_name.is_empty() {
            skipped += 1;
            continue;
        }

        // Check if contact already exists
        let existing = db.find_person_by_external_id("apple", &apple_id)?;

        if dry_run {
            let display = format_display_name(&given, &family, &org_name);
            if existing.is_some() {
                println!("  [UPDATE] {}", display);
            } else {
                println!("  [CREATE] {}", display);
            }
            continue;
        }

        match existing {
            Some(existing_person) => {
                update_person_from_contact(db, &existing_person, &contact)?;
                updated += 1;
            }
            None => {
                create_person_from_contact(db, &contact, &apple_id)?;
                created += 1;
            }
        }

        // Progress indicator every 100 contacts
        if (i + 1) % 100 == 0 {
            eprint!("\rProcessing... {}/{}", i + 1, total);
        }
    }

    if !dry_run && total >= 100 {
        eprint!("\r                                  \r"); // Clear progress line
    }

    println!();
    if dry_run {
        println!("Dry run complete.");
    } else {
        println!("Sync complete: {} created, {} updated, {} skipped",
                 created, updated, skipped);
    }

    // Offer to sync photos (even during dry-run)
    match prompt_photo_sync() {
        PhotoSyncMode::Skip => {}
        PhotoSyncMode::DryRun => sync_photos(db, &contacts, true)?,
        PhotoSyncMode::Sync if dry_run => {
            println!("Cannot sync photos during contact dry-run. Use 'd' to preview photo sync.");
        }
        PhotoSyncMode::Sync => sync_photos(db, &contacts, false)?,
    }

    Ok(())
}

fn request_authorization(store: &CNContactStore) -> Result<()> {
    let (tx, rx) = mpsc::channel();

    // Create block with correct signature for requestAccessForEntityType_completionHandler
    let block = RcBlock::new(move |granted: Bool, error: *mut NSError| {
        let error_msg = if error.is_null() {
            None
        } else {
            unsafe { Some((*error).localizedDescription().to_string()) }
        };
        let _ = tx.send((granted.as_bool(), error_msg));
    });

    unsafe {
        store.requestAccessForEntityType_completionHandler(
            CNEntityType::Contacts,
            &block,
        );
    }

    let (granted, error) = rx.recv().map_err(|_| anyhow!("Authorization request failed"))?;

    if !granted {
        if let Some(err) = error {
            return Err(anyhow!("Authorization denied: {}", err));
        }
        return Err(anyhow!("Authorization denied"));
    }

    Ok(())
}

fn fetch_all_contacts(store: &CNContactStore) -> Result<Retained<NSArray<CNContact>>> {
    let keys = create_keys_array();

    // Use predicateWithValue(true) to get all contacts
    let predicate = NSPredicate::predicateWithValue(true);

    unsafe {
        store.unifiedContactsMatchingPredicate_keysToFetch_error(&predicate, &keys)
            .map_err(|e| anyhow!("Failed to fetch contacts: {}", e.localizedDescription()))
    }
}

fn create_keys_array() -> Retained<NSArray<objc2::runtime::ProtocolObject<dyn CNKeyDescriptor>>> {
    unsafe {
        // The key constants are NSStrings that conform to CNKeyDescriptor
        let keys: &[&NSString] = &[
            CNContactIdentifierKey,
            CNContactGivenNameKey,
            CNContactFamilyNameKey,
            CNContactMiddleNameKey,
            CNContactNamePrefixKey,
            CNContactNameSuffixKey,
            CNContactNicknameKey,
            CNContactOrganizationNameKey,
            CNContactJobTitleKey,
            CNContactEmailAddressesKey,
            CNContactPhoneNumbersKey,
            CNContactPostalAddressesKey,
            CNContactBirthdayKey,
            CNContactImageDataKey,
        ];

        // Create an NSArray from the keys - they're protocol objects for CNKeyDescriptor
        let mut ptrs: Vec<NonNull<objc2::runtime::ProtocolObject<dyn CNKeyDescriptor>>> = Vec::new();
        for key in keys {
            // Cast NSString to CNKeyDescriptor protocol object
            let ptr = *key as *const NSString as *const objc2::runtime::ProtocolObject<dyn CNKeyDescriptor>;
            ptrs.push(NonNull::new_unchecked(ptr as *mut _));
        }

        NSArray::from_retained_slice(
            &keys.iter().map(|&k| {
                // NSString keys are CNKeyDescriptor-conforming
                let obj: Retained<objc2::runtime::ProtocolObject<dyn CNKeyDescriptor>> =
                    Retained::cast_unchecked(k.retain());
                obj
            }).collect::<Vec<_>>()
        )
    }
}

fn get_contact_identifier(contact: &CNContact) -> String {
    unsafe { contact.identifier().to_string() }
}

fn get_contact_given_name(contact: &CNContact) -> String {
    unsafe { contact.givenName().to_string() }
}

fn get_contact_family_name(contact: &CNContact) -> String {
    unsafe { contact.familyName().to_string() }
}

fn get_contact_organization(contact: &CNContact) -> String {
    unsafe { contact.organizationName().to_string() }
}

fn format_display_name(given: &str, family: &str, org: &str) -> String {
    let mut parts = Vec::new();
    if !given.is_empty() {
        parts.push(given);
    }
    if !family.is_empty() {
        parts.push(family);
    }
    if parts.is_empty() && !org.is_empty() {
        return org.to_string();
    }
    parts.join(" ")
}

fn create_person_from_contact(db: &Database, contact: &CNContact, apple_id: &str) -> Result<()> {
    let mut person = Person::new();

    // Name fields
    person.name_given = non_empty_string(unsafe { contact.givenName().to_string() });
    person.name_family = non_empty_string(unsafe { contact.familyName().to_string() });
    person.name_middle = non_empty_string(unsafe { contact.middleName().to_string() });
    person.name_prefix = non_empty_string(unsafe { contact.namePrefix().to_string() });
    person.name_suffix = non_empty_string(unsafe { contact.nameSuffix().to_string() });
    person.name_nickname = non_empty_string(unsafe { contact.nickname().to_string() });

    // Store Apple ID for re-sync
    let mut external_ids = HashMap::new();
    external_ids.insert("apple".to_string(), apple_id.to_string());
    person.external_ids = Some(serde_json::to_string(&external_ids)?);

    person.compute_names();
    db.insert_person(&person)?;

    // Emails
    import_emails(db, contact, person.id)?;

    // Phones
    import_phones(db, contact, person.id)?;

    // Addresses
    import_addresses(db, contact, person.id)?;

    // Birthday
    import_birthday(db, contact, person.id)?;

    // Organization
    import_organization(db, contact, person.id)?;

    Ok(())
}

fn update_person_from_contact(db: &Database, person: &Person, contact: &CNContact) -> Result<()> {
    let mut updated = person.clone();

    // Update name fields
    updated.name_given = non_empty_string(unsafe { contact.givenName().to_string() });
    updated.name_family = non_empty_string(unsafe { contact.familyName().to_string() });
    updated.name_middle = non_empty_string(unsafe { contact.middleName().to_string() });
    updated.name_prefix = non_empty_string(unsafe { contact.namePrefix().to_string() });
    updated.name_suffix = non_empty_string(unsafe { contact.nameSuffix().to_string() });
    updated.name_nickname = non_empty_string(unsafe { contact.nickname().to_string() });

    updated.compute_names();
    db.update_person(&updated)?;

    // Clear and re-import related data
    db.delete_emails_for_person(person.id)?;
    db.delete_phones_for_person(person.id)?;
    db.delete_addresses_for_person(person.id)?;

    import_emails(db, contact, person.id)?;
    import_phones(db, contact, person.id)?;
    import_addresses(db, contact, person.id)?;

    Ok(())
}

fn import_emails(db: &Database, contact: &CNContact, person_id: uuid::Uuid) -> Result<()> {
    unsafe {
        let emails = contact.emailAddresses();
        let count = emails.count();

        for i in 0..count {
            let labeled = emails.objectAtIndex(i);
            let value_obj = labeled.value();

            // The value should be an NSString
            if let Some(ns_str) = value_obj.downcast_ref::<NSString>() {
                let addr_str = ns_str.to_string();
                if !addr_str.is_empty() {
                    let mut email = Email::new(person_id, addr_str);
                    email.is_primary = i == 0;

                    if let Some(label) = labeled.label() {
                        email.email_type = email_type_from_label(&label.to_string());
                    }

                    db.insert_email(&email)?;
                }
            }
        }
    }
    Ok(())
}

fn import_phones(db: &Database, contact: &CNContact, person_id: uuid::Uuid) -> Result<()> {
    unsafe {
        let phones = contact.phoneNumbers();
        let count = phones.count();

        for i in 0..count {
            let labeled = phones.objectAtIndex(i);
            let value_obj = labeled.value();

            if let Some(phone_num) = value_obj.downcast_ref::<CNPhoneNumber>() {
                let num_str = phone_num.stringValue().to_string();
                if !num_str.is_empty() {
                    let mut phone = Phone::new(person_id, num_str);
                    phone.is_primary = i == 0;

                    if let Some(label) = labeled.label() {
                        phone.phone_type = phone_type_from_label(&label.to_string());
                    }

                    db.insert_phone(&phone)?;
                }
            }
        }
    }
    Ok(())
}

fn import_addresses(db: &Database, contact: &CNContact, person_id: uuid::Uuid) -> Result<()> {
    unsafe {
        let addresses = contact.postalAddresses();
        let count = addresses.count();

        for i in 0..count {
            let labeled = addresses.objectAtIndex(i);
            let value_obj = labeled.value();

            if let Some(postal) = value_obj.downcast_ref::<CNPostalAddress>() {
                let mut addr = Address::new(person_id);

                addr.street = non_empty_string(postal.street().to_string());
                addr.city = non_empty_string(postal.city().to_string());
                addr.state = non_empty_string(postal.state().to_string());
                addr.postal_code = non_empty_string(postal.postalCode().to_string());
                addr.country = non_empty_string(postal.country().to_string());
                addr.is_primary = i == 0;

                if let Some(label) = labeled.label() {
                    addr.address_type = address_type_from_label(&label.to_string());
                }

                // Only insert if we have at least one field
                if addr.street.is_some() || addr.city.is_some() || addr.state.is_some() {
                    db.insert_address(&addr)?;
                }
            }
        }
    }
    Ok(())
}

fn import_birthday(db: &Database, contact: &CNContact, person_id: uuid::Uuid) -> Result<()> {
    unsafe {
        if let Some(birthday) = contact.birthday() {
            let year = birthday.year();
            let month = birthday.month();
            let day = birthday.day();

            // NSDateComponentUndefined is represented as max value
            let year_known = year != isize::MAX && year > 0;

            if month > 0 && day > 0 {
                let date_str = if year_known {
                    format!("{:04}-{:02}-{:02}", year, month, day)
                } else {
                    format!("0000-{:02}-{:02}", month, day)
                };

                let mut special_date = SpecialDate::new(person_id, date_str, DateType::Birthday);
                special_date.year_known = year_known;

                db.insert_special_date(&special_date)?;
            }
        }
    }
    Ok(())
}

fn import_organization(db: &Database, contact: &CNContact, person_id: uuid::Uuid) -> Result<()> {
    let org_name = unsafe { contact.organizationName().to_string() };

    if org_name.is_empty() {
        return Ok(());
    }

    let org = db.get_or_create_organization(&org_name)?;

    let mut po = PersonOrganization::new(person_id, org.id);
    po.title = non_empty_string(unsafe { contact.jobTitle().to_string() });
    po.is_primary = true;

    db.insert_person_organization(&po)?;

    Ok(())
}

fn non_empty_string(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn email_type_from_label(label: &str) -> EmailType {
    let lower = label.to_lowercase();
    if lower.contains("work") {
        EmailType::Work
    } else if lower.contains("home") {
        EmailType::Personal
    } else {
        EmailType::Other
    }
}

fn phone_type_from_label(label: &str) -> PhoneType {
    let lower = label.to_lowercase();
    if lower.contains("mobile") || lower.contains("iphone") {
        PhoneType::Mobile
    } else if lower.contains("home") {
        PhoneType::Home
    } else if lower.contains("work") {
        PhoneType::Work
    } else if lower.contains("fax") {
        PhoneType::Fax
    } else {
        PhoneType::Other
    }
}

fn address_type_from_label(label: &str) -> AddressType {
    let lower = label.to_lowercase();
    if lower.contains("work") {
        AddressType::Work
    } else if lower.contains("home") {
        AddressType::Home
    } else {
        AddressType::Other
    }
}

/// Delete a contact from macOS Contacts by Apple identifier
pub fn delete_from_macos_contacts(apple_id: &str) -> Result<()> {
    let store = unsafe { CNContactStore::new() };

    // Check authorization
    let status = unsafe { CNContactStore::authorizationStatusForEntityType(CNEntityType::Contacts) };

    if status != CNAuthorizationStatus::Authorized {
        return Err(anyhow!("Not authorized to modify macOS Contacts"));
    }

    // Fetch the contact by identifier
    let keys = create_keys_array();
    let contact = unsafe {
        store.unifiedContactWithIdentifier_keysToFetch_error(
            &NSString::from_str(apple_id),
            &keys,
        )
    };

    let contact = match contact {
        Ok(c) => c,
        Err(_) => {
            // Contact not found in macOS - that's fine, nothing to delete
            return Ok(());
        }
    };

    // Create a mutable copy and delete request
    unsafe {
        let mutable_contact: Retained<CNMutableContact> = contact.mutableCopy();
        let save_request = CNSaveRequest::new();
        save_request.deleteContact(&mutable_contact);

        store.executeSaveRequest_error(&save_request)
            .map_err(|e| anyhow!("Failed to delete contact from macOS: {}", e.localizedDescription()))?;
    }

    Ok(())
}

/// Batch delete multiple contacts from macOS Contacts
/// Returns (succeeded, failed) counts
pub fn delete_from_macos_contacts_batch(apple_ids: &[String]) -> Result<(usize, usize)> {
    if apple_ids.is_empty() {
        return Ok((0, 0));
    }

    let store = unsafe { CNContactStore::new() };

    // Check authorization
    let status = unsafe { CNContactStore::authorizationStatusForEntityType(CNEntityType::Contacts) };

    if status != CNAuthorizationStatus::Authorized {
        return Err(anyhow!("Not authorized to modify macOS Contacts"));
    }

    let keys = create_keys_array();
    let save_request = unsafe { CNSaveRequest::new() };

    let mut found = 0;
    let mut not_found = 0;

    // Add all contacts to the save request
    for apple_id in apple_ids {
        let contact = unsafe {
            store.unifiedContactWithIdentifier_keysToFetch_error(
                &NSString::from_str(apple_id),
                &keys,
            )
        };

        match contact {
            Ok(c) => {
                unsafe {
                    let mutable_contact: Retained<CNMutableContact> = c.mutableCopy();
                    save_request.deleteContact(&mutable_contact);
                }
                found += 1;
            }
            Err(_) => {
                // Contact not found - that's fine
                not_found += 1;
            }
        }
    }

    // Execute all deletes in one batch
    if found > 0 {
        unsafe {
            store.executeSaveRequest_error(&save_request)
                .map_err(|e| anyhow!("Failed to delete contacts from macOS: {}", e.localizedDescription()))?;
        }
    }

    Ok((found, not_found))
}

/// Extract Apple ID from a Person's external_ids JSON field
pub fn get_apple_id(person: &Person) -> Option<String> {
    let external_ids = person.external_ids.as_ref()?;
    let parsed: HashMap<String, String> = serde_json::from_str(external_ids).ok()?;
    parsed.get("apple").cloned()
}

/// Photo sync mode chosen by user
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhotoSyncMode {
    Skip,
    DryRun,
    Sync,
}

/// Prompt user to sync photos
fn prompt_photo_sync() -> PhotoSyncMode {
    use std::io::{self, Write};

    print!("\nSync photos? [y/N/d(ry-run)]: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return PhotoSyncMode::Skip;
    }

    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => PhotoSyncMode::Sync,
        "d" | "dry" | "dry-run" => PhotoSyncMode::DryRun,
        _ => PhotoSyncMode::Skip,
    }
}

/// Sync photos from macOS Contacts
fn sync_photos(db: &Database, contacts: &NSArray<CNContact>, dry_run: bool) -> Result<()> {
    use photo_utils::SaveResult;

    let total = contacts.count();

    if dry_run {
        println!("Photo sync dry run...");
    } else {
        println!("Syncing photos...");
    }

    let mut synced = 0;
    let mut unchanged = 0;
    let mut no_photo = 0;

    for i in 0..total {
        let contact = contacts.objectAtIndex(i);
        let apple_id = get_contact_identifier(&contact);

        if apple_id.is_empty() {
            continue;
        }

        // Get image data from contact
        let image_data = unsafe { contact.imageData() };

        let Some(data) = image_data else {
            no_photo += 1;
            continue;
        };

        // Find the person in our database
        let person = match db.find_person_by_external_id("apple", &apple_id)? {
            Some(p) => p,
            None => {
                no_photo += 1;
                continue;
            }
        };

        // Convert NSData to Vec<u8>
        let image_bytes = data.to_vec();

        if image_bytes.is_empty() {
            no_photo += 1;
            continue;
        }

        if dry_run {
            // Check what would happen without saving
            match photo_utils::would_photo_change(person.id, &image_bytes) {
                SaveResult::Saved => synced += 1,
                SaveResult::Unchanged => unchanged += 1,
                SaveResult::Invalid => no_photo += 1,
            }
        } else {
            // Save with change detection
            match photo_utils::save_photo_bytes_if_changed(person.id, &image_bytes) {
                SaveResult::Saved => synced += 1,
                SaveResult::Unchanged => unchanged += 1,
                SaveResult::Invalid => no_photo += 1,
            }
        }

        // Progress indicator every 50 contacts
        if (synced + unchanged) % 50 == 0 && (synced + unchanged) > 0 {
            eprint!("\rProcessing photos... {}", synced + unchanged);
        }
    }

    if synced + unchanged >= 50 {
        eprint!("\r                          \r"); // Clear progress line
    }

    if dry_run {
        println!(
            "Dry run complete: {} would sync, {} unchanged, {} without photos",
            synced, unchanged, no_photo
        );
    } else {
        println!(
            "Photo sync complete: {} synced, {} unchanged, {} without photos",
            synced, unchanged, no_photo
        );
    }
    Ok(())
}
